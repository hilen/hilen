use anyhow::{Context, Result, bail};
use log::{error, info, warn};
use openxr as xr;
use wgpu::{
    Adapter, Backends, Device, Instance, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    TextureUses, TextureView, TextureViewDescriptor,
    hal::{api::Dx12, dx12},
};
use windows::{
    Win32::Graphics::{
        Direct3D12::ID3D12Resource,
        Dxgi::Common::{DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_FORMAT_R8G8B8A8_UNORM},
    },
    core::Interface,
};

use super::{
    STATE,
    input::{Input, located},
    pose::Rig,
};
use crate::{
    deps::refs::main_lock::MainLock,
    gm::flat::Size,
    scene::{Camera, Fov, SceneManager},
    scene_drawer::SceneDrawer,
    window::{Window, surface_texture_format},
};

const STEREO: xr::ViewConfigurationType = xr::ViewConfigurationType::PRIMARY_STEREO;

static RUNTIME: MainLock<Runtime> = MainLock::new();

/// `OpenXR` before the GPU device: the runtime, the headset and the GPU it
/// is plugged into.
pub(crate) struct Boot {
    instance: xr::Instance,
    system:   xr::SystemId,
    luid:     xr::sys::platform::LUID,
}

/// `OpenXR` started, or None with the reason in the log when there is no
/// runtime or no headset. An app with no headset runs as before.
pub(crate) fn boot() -> Option<Boot> {
    match Boot::start() {
        Ok(boot) => Some(boot),
        Err(err) => {
            info!("No VR: {err:#}");
            None
        }
    }
}

impl Boot {
    fn start() -> Result<Self> {
        let entry = xr::Entry::linked(&()).context("OpenXR loader")?;
        let available = entry.enumerate_extensions().context("OpenXR extensions")?;
        if !available.khr_d3d12_enable {
            bail!("the OpenXR runtime has no D3D12 support");
        }
        let mut extensions = xr::ExtensionSet::default();
        extensions.khr_d3d12_enable = true;
        let name = crate::log_file::app_name().unwrap_or_else(|_| "hilen".to_string());
        let instance = entry
            .create_instance(
                &xr::ApplicationInfo {
                    application_name:    &name,
                    application_version: 0,
                    engine_name:         "hilen",
                    engine_version:      0,
                    api_version:         xr::Version::new(1, 0, 0),
                },
                &extensions,
                &[],
                &(),
            )
            .context("OpenXR instance")?;
        let properties = instance.properties()?;
        info!(
            "OpenXR runtime: {} {}",
            properties.runtime_name, properties.runtime_version
        );
        let system = instance.system(xr::FormFactor::HEAD_MOUNTED_DISPLAY).context("no headset")?;
        let luid = instance.graphics_requirements::<xr::D3D12>(system)?.adapter_luid;
        info!("Headset: {}", instance.system_properties(system)?.system_name);
        Ok(Self {
            instance,
            system,
            luid,
        })
    }
}

/// The DX12 adapter the headset is plugged into, `fallback` when no adapter
/// has its id. A machine with 1 GPU has no other choice.
pub(crate) async fn adapter_for(instance: &Instance, boot: Option<&Boot>, fallback: Adapter) -> Adapter {
    let Some(boot) = boot else {
        return fallback;
    };
    let wanted = (boot.luid.LowPart, boot.luid.HighPart);
    for adapter in instance.enumerate_adapters(Backends::DX12).await {
        // SAFETY: the guard lives only for the read of the description.
        let luid = unsafe { adapter.as_hal::<Dx12>() }
            .and_then(|hal| unsafe { hal.raw_adapter().GetDesc2() }.ok())
            .map(|desc| (desc.AdapterLuid.LowPart, desc.AdapterLuid.HighPart));
        if luid == Some(wanted) {
            info!("VR on the GPU of the headset: {}", adapter.get_info().name);
            return adapter;
        }
    }
    warn!("No DX12 adapter has the id of the headset GPU, VR may not start");
    fallback
}

/// Makes the `OpenXR` session on `device`. A failure leaves the app with no
/// VR and says why in the log.
pub(crate) fn start(boot: Boot, device: &Device) {
    match Runtime::new(boot, device) {
        Ok(runtime) => {
            RUNTIME.set(runtime);
        }
        Err(err) => error!("VR did not start: {err:#}"),
    }
}

/// Runs 1 frame of the headset, after the frame of the window. It waits
/// for the headset, which takes the frame rate of the app to its own.
pub(crate) fn frame() {
    let Some(runtime) = RUNTIME.try_get_mut() else {
        return;
    };
    let result = runtime.frame();
    let ended = match result {
        Ok(()) => runtime.exit,
        Err(err) => {
            error!("VR stopped: {err:#}");
            true
        }
    };
    if ended {
        let vsync = runtime.window_vsync;
        RUNTIME.clear();
        *STATE.get_mut() = super::State::default();
        Window::set_vsync(vsync);
        info!("VR ended");
    }
}

struct Runtime {
    instance:     xr::Instance,
    system:       xr::SystemId,
    blend:        xr::EnvironmentBlendMode,
    session:      xr::Session<xr::D3D12>,
    waiter:       xr::FrameWaiter,
    stream:       xr::FrameStream<xr::D3D12>,
    /// The space of the headset, its origin where the head was when VR
    /// started, see `Rig`.
    local:        xr::Space,
    head:         xr::Space,
    input:        Input,
    eyes:         Option<[Eye; 2]>,
    events:       xr::EventDataBuffer,
    running:      bool,
    exit:         bool,
    /// The vsync of the window before VR turned it off. A window that
    /// waits for its own screen would halve the frames of the headset.
    window_vsync: bool,
}

impl Runtime {
    fn new(boot: Boot, device: &Device) -> Result<Self> {
        let Boot { instance, system, .. } = boot;
        // SAFETY: the guard lives only while the 2 pointers are read. The
        // device and its queue live as long as the app, longer than the
        // session that holds them.
        let (raw_device, raw_queue) = {
            let hal = unsafe { device.as_hal::<Dx12>() }.context("the GPU device is not DX12")?;
            (hal.raw_device().as_raw(), hal.raw_queue().as_raw())
        };
        // SAFETY: the device and the queue are live DX12 objects of the
        // adapter that `graphics_requirements` named.
        let (session, waiter, stream) = unsafe {
            instance.create_session::<xr::D3D12>(
                system,
                &xr::d3d::SessionCreateInfoD3D12 {
                    device: raw_device,
                    queue:  raw_queue,
                },
            )
        }
        .context("OpenXR session")?;
        let blend = *instance
            .enumerate_environment_blend_modes(system, STEREO)?
            .first()
            .context("no blend mode")?;
        let local = session.create_reference_space(xr::ReferenceSpaceType::LOCAL, xr::Posef::IDENTITY)?;
        let head = session.create_reference_space(xr::ReferenceSpaceType::VIEW, xr::Posef::IDENTITY)?;
        let input = Input::new(&instance, &session)?;
        info!("VR session made, waiting for the headset");
        Ok(Self {
            instance,
            system,
            blend,
            session,
            waiter,
            stream,
            local,
            head,
            input,
            eyes: None,
            events: xr::EventDataBuffer::new(),
            running: false,
            exit: false,
            window_vsync: Window::vsync(),
        })
    }

    fn frame(&mut self) -> Result<()> {
        self.poll()?;
        if !self.running {
            return Ok(());
        }
        let state = self.waiter.wait()?;
        self.stream.begin()?;
        let time = state.predicted_display_time;

        if SceneManager::no_scene() {
            *STATE.get_mut() = super::State {
                running: true,
                ..Default::default()
            };
            self.stream.end(time, self.blend, &[])?;
            return Ok(());
        }

        let scene_camera = SceneManager::scene().camera;
        let rig = Rig::of(&scene_camera);
        let hands = self.input.read(&self.session, &self.local, time, rig)?;
        let head = located(&self.head, &self.local, time)?.map(|pose| rig.world(pose));
        *STATE.get_mut() = super::State {
            running: true,
            head,
            hands,
        };

        if !state.should_render {
            self.stream.end(time, self.blend, &[])?;
            return Ok(());
        }

        let (_, views) = self.session.locate_views(STEREO, time, &self.local)?;
        if views.len() != 2 {
            bail!("the headset gave {} views, not 2", views.len());
        }
        for (eye, view) in self.eyes()?.iter_mut().zip(&views) {
            let pose = rig.world(super::input::pose(view.pose));
            let fov = Fov {
                left:  view.fov.angle_left,
                right: view.fov.angle_right,
                up:    view.fov.angle_up,
                down:  view.fov.angle_down,
            };
            let camera = Camera {
                near: scene_camera.near,
                far: scene_camera.far,
                ..Camera::from_pose(pose.position, pose.rotation, fov)
            };
            let image = eye.swapchain.acquire_image()? as usize;
            eye.swapchain.wait_image(xr::Duration::INFINITE)?;
            SceneDrawer::draw_eye(&camera, &eye.views[image], eye.size);
            eye.swapchain.release_image()?;
        }

        let eyes = self.eyes.as_ref().expect("made above");
        let projection_views = [0, 1].map(|i| {
            let eye = &eyes[i];
            xr::CompositionLayerProjectionView::new()
                .pose(views[i].pose)
                .fov(views[i].fov)
                .sub_image(xr::SwapchainSubImage::new().swapchain(&eye.swapchain).image_rect(eye.rect()))
        });
        let layer = xr::CompositionLayerProjection::new()
            .space(&self.local)
            .views(&projection_views);
        self.stream.end(time, self.blend, &[&layer])?;
        Ok(())
    }

    /// Takes the events of the runtime. The session runs between READY and
    /// STOPPING, and EXITING or a lost runtime ends VR.
    fn poll(&mut self) -> Result<()> {
        while let Some(event) = self.instance.poll_event(&mut self.events)? {
            match event {
                xr::Event::SessionStateChanged(change) => {
                    let state = change.state();
                    info!("VR session state: {state:?}");
                    match state {
                        xr::SessionState::READY => {
                            self.session.begin(STEREO)?;
                            self.running = true;
                            Window::set_vsync(false);
                        }
                        xr::SessionState::STOPPING => {
                            self.session.end()?;
                            self.running = false;
                            *STATE.get_mut() = super::State::default();
                            Window::set_vsync(self.window_vsync);
                        }
                        xr::SessionState::EXITING | xr::SessionState::LOSS_PENDING => {
                            self.exit = true;
                        }
                        _ => {}
                    }
                }
                xr::Event::InstanceLossPending(_) => self.exit = true,
                xr::Event::EventsLost(lost) => warn!("VR lost {} events", lost.lost_event_count()),
                _ => {}
            }
        }
        Ok(())
    }

    /// The images of both eyes, made at the first frame the headset shows.
    fn eyes(&mut self) -> Result<&mut [Eye; 2]> {
        if self.eyes.is_none() {
            let configs = self.instance.enumerate_view_configuration_views(self.system, STEREO)?;
            if configs.len() != 2 {
                bail!("the headset has {} views, not 2", configs.len());
            }
            let format = surface_texture_format();
            let wanted = match format {
                TextureFormat::Bgra8Unorm => DXGI_FORMAT_B8G8R8A8_UNORM,
                TextureFormat::Rgba8Unorm => DXGI_FORMAT_R8G8B8A8_UNORM,
                other => bail!("VR has no DXGI format for {other:?}"),
            }
            .0
            .cast_unsigned();
            let offered = self.session.enumerate_swapchain_formats()?;
            if !offered.contains(&wanted) {
                bail!("the runtime offers no {format:?} eye images, only DXGI formats {offered:?}");
            }
            let left = Eye::new(&self.session, &configs[0], wanted, format)?;
            let right = Eye::new(&self.session, &configs[1], wanted, format)?;
            info!(
                "VR eye images of {}x{}, {format:?}",
                left.size.width, left.size.height
            );
            self.eyes = Some([left, right]);
        }
        Ok(self.eyes.as_mut().expect("just made"))
    }
}

/// The images 1 eye is drawn into, the runtime's own textures as wgpu
/// textures.
struct Eye {
    swapchain: xr::Swapchain<xr::D3D12>,
    views:     Vec<TextureView>,
    size:      Size<u32>,
}

impl Eye {
    fn new(
        session: &xr::Session<xr::D3D12>,
        config: &xr::ViewConfigurationView,
        dxgi_format: u32,
        format: TextureFormat,
    ) -> Result<Self> {
        let size = Size::new(
            config.recommended_image_rect_width,
            config.recommended_image_rect_height,
        );
        let swapchain = session.create_swapchain(&xr::SwapchainCreateInfo {
            create_flags: xr::SwapchainCreateFlags::EMPTY,
            usage_flags:  xr::SwapchainUsageFlags::COLOR_ATTACHMENT,
            format:       dxgi_format,
            sample_count: 1,
            width:        size.width,
            height:       size.height,
            face_count:   1,
            array_size:   1,
            mip_count:    1,
        })?;
        let extent = wgpu::Extent3d {
            width:                 size.width,
            height:                size.height,
            depth_or_array_layers: 1,
        };
        let descriptor = TextureDescriptor {
            label: Some("xr_eye"),
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format,
            usage: TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        };
        let device = Window::device();
        let mut views = vec![];
        for raw in swapchain.enumerate_images()? {
            // SAFETY: the runtime keeps the image alive while the swapchain
            // lives, and the clone holds its own reference on top.
            let resource = unsafe { ID3D12Resource::from_raw_borrowed(&raw) }
                .context("the runtime gave a null eye image")?
                .clone();
            // SAFETY: the image is a 2D render target of this size and
            // format, made by the runtime on the device of the session.
            let texture = unsafe {
                let hal =
                    dx12::Device::texture_from_raw(resource, format, TextureDimension::D2, extent, 1, 1);
                // OpenXR hands a D3D12 eye image over as a render target
                // and wants it back as one, the only use it gets here.
                device.create_texture_from_hal::<Dx12>(hal, &descriptor, TextureUses::COLOR_TARGET)
            };
            views.push(texture.create_view(&TextureViewDescriptor::default()));
        }
        Ok(Self {
            swapchain,
            views,
            size,
        })
    }

    fn rect(&self) -> xr::Rect2Di {
        xr::Rect2Di {
            offset: xr::Offset2Di { x: 0, y: 0 },
            extent: xr::Extent2Di {
                width:  self.size.width.cast_signed(),
                height: self.size.height.cast_signed(),
            },
        }
    }
}
