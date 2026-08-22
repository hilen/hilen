use wgpu::BindGroup;

#[derive(Debug)]
pub(crate) struct ImageBind {
    bind: BindGroup,
}

#[cfg(wasm)]
unsafe impl Send for ImageBind {}

impl ImageBind {
    pub(crate) fn get(&self) -> &BindGroup {
        &self.bind
    }
}

impl From<BindGroup> for ImageBind {
    fn from(bind: BindGroup) -> Self {
        Self { bind }
    }
}
