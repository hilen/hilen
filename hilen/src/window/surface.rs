use std::sync::Arc;

use anyhow::Result;
use log::info;
use wgpu::{Adapter, Device, Instance, SurfaceConfiguration};
use winit::window::Window;

pub(crate) struct Surface {
    pub presentable: wgpu::Surface<'static>,
}

impl Surface {
    pub fn new(
        instance: &Instance,
        adapter: &Adapter,
        device: &Device,
        config: SurfaceConfiguration,
        window: Arc<Window>,
    ) -> Result<Self> {
        if config.width == 0 || config.height == 0 {
            panic!("Invalid surface size: {config:?}")
        }

        let surface = instance.create_surface(window.clone())?;

        let surface_caps = surface.get_capabilities(adapter);

        info!("surface_caps: {surface_caps:?}");
        info!("limits: {:?}", adapter.limits());

        surface.configure(device, &config);

        Ok(Self { presentable: surface })
    }
}
