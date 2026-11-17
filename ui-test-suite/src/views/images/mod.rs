/// The download test talks to a server inside the test, a browser page
/// cannot open a port.
#[cfg(not_wasm)]
mod image_download;
mod image_downscale;
mod image_flip;
mod image_on_view;
mod image_view;
mod image_view_svg;
mod svg_scale;
