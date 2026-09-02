use wgpu::{BindGroup, Sampler, TextureView};

/// The sampled side of an image: the bind group the rect pipelines set
/// and the view and sampler a pipeline with a wider layout, like the
/// mesh one with its two textures, binds itself.
#[derive(Debug)]
pub(crate) struct ImageBind {
    bind:    BindGroup,
    view:    TextureView,
    sampler: Sampler,
}

#[cfg(wasm)]
unsafe impl Send for ImageBind {}

impl ImageBind {
    pub(crate) fn new(bind: BindGroup, view: TextureView, sampler: Sampler) -> Self {
        Self { bind, view, sampler }
    }

    pub(crate) fn get(&self) -> &BindGroup {
        &self.bind
    }

    pub(crate) fn view(&self) -> &TextureView {
        &self.view
    }

    pub(crate) fn sampler(&self) -> &Sampler {
        &self.sampler
    }
}
