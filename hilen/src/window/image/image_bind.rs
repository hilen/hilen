#[cfg(feature = "scene")]
use wgpu::Sampler;
use wgpu::{BindGroup, TextureView};

/// The sampled side of an image: the bind group the rect pipelines set
/// and the view and sampler a pipeline with a wider layout, like the
/// mesh one with its two textures, binds itself. The view also lets
/// `Image::set_filter` bind the texture again with another sampler.
#[derive(Debug)]
pub(crate) struct ImageBind {
    pub(crate) bind:    BindGroup,
    pub(crate) view:    TextureView,
    #[cfg(feature = "scene")]
    pub(crate) sampler: Sampler,
}

#[cfg(wasm)]
unsafe impl Send for ImageBind {}
