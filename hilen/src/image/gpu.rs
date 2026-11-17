//! The GPU side of pictures. `Image` and `Svg` hold what describes a
//! picture, its size, its filter and the svg tree. The textures and bind
//! groups live here by the id of the picture, so the views build with no
//! GPU, see `docs/roadmap.md`.

use std::collections::HashMap;
#[cfg(any(feature = "camera", all(feature = "video", ffmpeg), feature = "scene"))]
use std::convert::Infallible;

use wgpu::{
    BindGroup, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
    BindingResource, BindingType, Sampler, SamplerBindingType, ShaderStages, TextureSampleType, TextureView,
    TextureViewDimension,
};

#[cfg(any(feature = "camera", all(feature = "video", ffmpeg), feature = "scene"))]
use crate::deps::refs::{Weak, manage::DataManager};
use crate::{
    deps::{
        hreads::{is_main_thread, on_main},
        refs::main_lock::MainLock,
    },
    gm::flat::Size,
    image::{Image, ImageBind, ImageFilter, Svg, Texture},
    window::Window,
};

/// What the GPU has of 1 image.
pub(crate) struct ImageGpu {
    bind:            ImageBind,
    /// A bundled jpeg in a browser, whose pixels the page decodes later.
    /// `bind` is transparent until then.
    #[cfg(wasm)]
    pub(crate) late: Option<crate::image::late::Late>,
}

static IMAGES: MainLock<HashMap<u64, ImageGpu>> = MainLock::new();

impl ImageGpu {
    pub(crate) fn new(bind: ImageBind) -> Self {
        Self {
            bind,
            #[cfg(wasm)]
            late: None,
        }
    }

    pub(crate) fn store(self, id: u64) {
        IMAGES.get_mut().insert(id, self);
    }

    /// Drops the entry of a picture that is gone. A picture can go on any
    /// thread, the textures only on the main one.
    pub(crate) fn forget(id: u64) {
        if is_main_thread() {
            IMAGES.get_mut().remove(&id);
        } else {
            on_main(move || {
                IMAGES.get_mut().remove(&id);
            });
        }
    }

    /// The bind of the pixels, a late one once it landed.
    fn current(&self) -> &ImageBind {
        #[cfg(wasm)]
        if let Some(bind) = self.late.as_ref().and_then(crate::image::late::Late::get) {
            return bind;
        }
        &self.bind
    }
}

impl Image {
    fn gpu(&self) -> &'static mut ImageGpu {
        IMAGES
            .get_mut()
            .get_mut(&self.id)
            .expect("an image has its GPU entry while it lives")
    }

    pub(crate) fn bind(&self) -> &BindGroup {
        #[cfg(wasm)]
        if let Some(svg) = &self.svg {
            return svg.fixed_bind();
        }
        &self.gpu().current().bind
    }

    #[cfg(any(feature = "scene", all(feature = "video", ffmpeg)))]
    pub(crate) fn view(&self) -> &TextureView {
        &self.gpu().current().view
    }

    #[cfg(feature = "scene")]
    pub(crate) fn sampler(&self) -> &Sampler {
        &self.gpu().current().sampler
    }

    /// The texture samples with `filter` from now on.
    pub(crate) fn gpu_filter(&self, filter: ImageFilter) {
        let gpu = self.gpu();
        let view: TextureView = gpu.bind.view.clone();
        gpu.bind = Self::bind_view(&view, &Texture::image_sampler(filter));
        #[cfg(wasm)]
        if let Some(late) = &mut gpu.late {
            late.set_filter(filter);
        }
    }

    /// Pictures of this image drawn on `frame` or later that the browser
    /// still makes, see `late::pictures_pending`.
    #[cfg(wasm)]
    pub(crate) fn pending_since(&self, frame: u64) -> usize {
        let svg = self.svg.as_ref().map_or(0, |svg| svg.pending_since(frame));
        let late = self.gpu().late.as_ref().is_some_and(|late| {
            late.is_pending() && self.drawn_at.load(std::sync::atomic::Ordering::Relaxed) >= frame
        });
        svg + usize::from(late)
    }

    /// Whether the pixels are here, false while a browser still makes a
    /// bundled jpeg. The damage of a view changes when they land.
    #[cfg_attr(not_wasm, allow(clippy::unused_self))]
    pub(crate) fn pixels_ready(&self) -> bool {
        #[cfg(wasm)]
        return self.gpu().late.as_ref().is_none_or(|late| late.get().is_some());
        #[cfg(not_wasm)]
        true
    }
}

/// The textures and bind groups of a picture made on the GPU.
impl Image {
    pub(crate) fn from_texture(texture: &Texture, svg: Option<Svg>) -> Self {
        let gpu = ImageGpu::new(Self::bind_texture(texture));
        Self::with_gpu(texture.size, texture.channels, svg, gpu)
    }

    /// An image whose texture the drawer keeps from now on.
    fn with_gpu(size: Size<u32>, channels: u8, svg: Option<Svg>, gpu: ImageGpu) -> Self {
        let image = Self::described(size, channels, svg);
        gpu.store(image.id);
        image
    }

    /// An svg in a browser, whose rasters the page draws later. Its own
    /// texture is never drawn, `bind` hands out the fixed raster.
    #[cfg(wasm)]
    pub(crate) fn from_svg(svg: Svg) -> Self {
        let bind = Self::bind_texture(&crate::image::late::transparent_texture("svg_image"));
        Self::with_gpu(svg.fixed_size(), 4, Some(svg), ImageGpu::new(bind))
    }

    /// A bundled jpeg in a browser. The wasm has no jpeg decoder, the page
    /// decodes it, see `browser_decode`. The size comes from the header.
    #[cfg(wasm)]
    pub(crate) fn from_browser(name: &str, data: &[u8]) -> anyhow::Result<Self> {
        use crate::image::{browser_decode, image_header, late};

        let (width, height) = image_header::dimensions(data)
            .ok_or_else(|| anyhow::anyhow!("No size in the header of {name}"))?;
        let late = late::Late::default();
        let bytes = data.to_vec();
        late.start(name.to_string(), async move {
            browser_decode::decode(&bytes, 0).await
        });
        let mut gpu = ImageGpu::new(Self::bind_texture(&late::transparent_texture(name)));
        gpu.late = Some(late);
        Ok(Self::with_gpu(Size::new(width, height), 4, None, gpu))
    }

    pub(crate) fn bind_texture(texture: &Texture) -> ImageBind {
        Self::bind_view(&texture.view, &texture.sampler)
    }

    pub(crate) fn bind_view(view: &TextureView, sampler: &Sampler) -> ImageBind {
        let bind = Window::device().create_bind_group(&wgpu::BindGroupDescriptor {
            label:   "image_bind_group".into(),
            layout:  Self::uniform_layout(),
            entries: &[
                BindGroupEntry {
                    binding:  0,
                    resource: BindingResource::TextureView(view),
                },
                BindGroupEntry {
                    binding:  1,
                    resource: BindingResource::Sampler(sampler),
                },
            ],
        });
        ImageBind {
            bind,
            view: view.clone(),
            #[cfg(feature = "scene")]
            sampler: sampler.clone(),
        }
    }

    pub(crate) fn uniform_layout() -> &'static BindGroupLayout {
        static LAYOUT: MainLock<BindGroupLayout> = MainLock::new();
        LAYOUT.get_or_init(|| {
            Window::device().create_bind_group_layout(&BindGroupLayoutDescriptor {
                label:   "image_bind_group_layout".into(),
                entries: &[
                    BindGroupLayoutEntry {
                        binding:    0,
                        visibility: ShaderStages::FRAGMENT,
                        ty:         BindingType::Texture {
                            multisampled:   false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type:    TextureSampleType::Float { filterable: true },
                        },
                        count:      None,
                    },
                    BindGroupLayoutEntry {
                        binding:    1,
                        visibility: ShaderStages::FRAGMENT,
                        ty:         BindingType::Sampler(SamplerBindingType::Filtering),
                        count:      None,
                    },
                ],
            })
        })
    }

    /// A picture whose pixels change in place, like the live picture of a
    /// camera, RGBA with 1 level. `Texture::write_level_0` on the texture
    /// it gives puts new pixels in. Keyed like any managed image, free it
    /// by its name when it is done. The texture stays out of the image, a
    /// texture of WebGPU may not leave the main thread and an image may.
    #[cfg(feature = "camera")]
    pub(crate) fn stream(name: &str, size: Size<u32>) -> (Weak<Image>, wgpu::Texture) {
        let texture = Texture::stream(size, name);
        let image =
            Image::store_with_name::<Infallible>(name, || Ok(Self::from_texture(&texture, None))).unwrap();
        (image, texture.texture)
    }

    /// A blank image whose texture is also a render target, for a video frame
    /// converted on the GPU or a scene drawn into a picture. Keyed like any
    /// managed image, so the same key gives the same image back.
    #[cfg(any(all(feature = "video", ffmpeg), feature = "scene"))]
    pub(crate) fn render_target(name: &str, size: Size<u32>) -> Weak<Image> {
        Image::store_with_name::<Infallible>(name, || {
            Ok(Self::from_texture(&Texture::render_target(size, name), None))
        })
        .unwrap()
    }

    /// A blank image in the format of the surface that a scene resolves
    /// into straight, with no copy, see `SceneDrawer::window_picture`.
    #[cfg(feature = "scene")]
    pub(crate) fn surface_target(name: &str, size: Size<u32>) -> Weak<Image> {
        Image::store_with_name::<Infallible>(name, || {
            Ok(Self::from_texture(&Texture::pixel_target(size), None))
        })
        .unwrap()
    }
}
