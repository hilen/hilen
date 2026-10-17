#[cfg(wasm)]
mod browser_decode;
#[cfg(not_wasm)]
mod disk_cache;
mod gif;
mod image;
mod image_bind;
#[cfg(any(wasm, test))]
mod image_header;
#[cfg(wasm)]
pub(crate) mod late;
#[cfg(wasm)]
pub(crate) mod pending;
mod remote;
#[cfg(not_wasm)]
mod svg;
#[cfg(wasm)]
#[path = "svg_browser.rs"]
mod svg;
#[cfg(any(wasm, test))]
mod svg_size;
mod texture;
mod tinted_image;
mod to_image;

pub(crate) use gif::decode_gif;
pub use image::*;
pub(crate) use image_bind::*;
pub use svg::*;
pub use texture::*;
pub use tinted_image::*;
pub use to_image::*;
