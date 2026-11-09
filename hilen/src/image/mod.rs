//! Pictures: `Image`, `Svg` and the decode of their files. The textures
//! and bind groups are the GPU side, built only with the `gpu` feature.

#[cfg(wasm)]
mod browser_decode;
#[cfg(all(not_wasm, feature = "net"))]
mod disk_cache;
mod gif;
#[cfg(feature = "gpu")]
mod gpu;
mod image;
#[cfg(feature = "gpu")]
mod image_bind;
#[cfg(any(wasm, test))]
mod image_header;
#[cfg(all(wasm, feature = "gpu"))]
pub(crate) mod late;
#[cfg(wasm)]
pub(crate) mod pending;
mod raw;
#[cfg(feature = "net")]
mod remote;
#[cfg(not_wasm)]
mod svg;
#[cfg(wasm)]
#[path = "svg_browser.rs"]
mod svg;
#[cfg(all(wasm, feature = "gpu"))]
mod svg_browser_gpu;
#[cfg(all(not_wasm, feature = "gpu"))]
mod svg_gpu;
#[cfg(any(wasm, test))]
mod svg_size;
#[cfg(feature = "gpu")]
mod texture;
mod tinted_image;
mod to_image;

#[cfg(wasm)]
pub(crate) use browser_decode::decode_upright;
pub(crate) use gif::decode_gif;
#[cfg(feature = "gpu")]
pub(crate) use gpu::ImageGpu;
#[cfg(feature = "gpu")]
pub(crate) use image::next_gpu_id;
pub use image::*;
#[cfg(feature = "gpu")]
pub(crate) use image_bind::*;
pub use raw::*;
pub use svg::*;
#[cfg(feature = "gpu")]
pub use texture::*;
pub use tinted_image::*;
pub use to_image::*;
