use image::{ExtendedColorType, ImageEncoder, Rgba, RgbaImage, codecs::png::PngEncoder};

/// The 4 quarters of the test photo, so every place of the cut shows.
pub(crate) const TOP_LEFT: [u8; 4] = [224, 49, 49, 255];
pub(crate) const TOP_RIGHT: [u8; 4] = [28, 126, 214, 255];
pub(crate) const BOTTOM_LEFT: [u8; 4] = [47, 158, 68, 255];
pub(crate) const BOTTOM_RIGHT: [u8; 4] = [245, 159, 0, 255];

/// A png 600 wide and 400 high in 4 colored quarters.
pub(crate) fn quarters_png() -> Vec<u8> {
    let picture = RgbaImage::from_fn(600, 400, |x, y| {
        Rgba(match (x < 300, y < 200) {
            (true, true) => TOP_LEFT,
            (false, true) => TOP_RIGHT,
            (true, false) => BOTTOM_LEFT,
            (false, false) => BOTTOM_RIGHT,
        })
    });
    let mut png = Vec::new();
    PngEncoder::new(&mut png)
        .write_image(picture.as_raw(), 600, 400, ExtendedColorType::Rgba8)
        .unwrap();
    png
}
