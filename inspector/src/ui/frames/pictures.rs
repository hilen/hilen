//! The pictures of the frames screen, made off the main thread.

use anyhow::{Result, ensure};
use image::{ImageFormat, imageops::thumbnail, load_from_memory_with_format};

/// The color of a changed pixel. It is the same in both themes, it is
/// painted into the picture of another app and has to stand out there.
const CHANGED: [u8; 4] = [255, 0, 255, 255];

/// RGBA pixels.
pub struct Pixels {
    pub rgba:   Vec<u8>,
    pub width:  u32,
    pub height: u32,
}

/// The picture of a kept frame, it comes in the QOI format.
pub fn decode(qoi: &[u8]) -> Result<Pixels> {
    let picture = load_from_memory_with_format(qoi, ImageFormat::Qoi)?.into_rgba8();
    let (width, height) = picture.dimensions();
    Ok(Pixels {
        rgba: picture.into_raw(),
        width,
        height,
    })
}

/// The frame scaled to `height` pixels, with its own shape.
pub fn thumb(qoi: &[u8], height: u32) -> Result<Pixels> {
    let picture = load_from_memory_with_format(qoi, ImageFormat::Qoi)?.into_rgba8();
    ensure!(picture.height() > 0, "The frame has no pixels");
    let width = (picture.width() * height / picture.height()).max(1);
    let small = thumbnail(&picture, width, height);
    Ok(Pixels {
        rgba: small.into_raw(),
        width,
        height,
    })
}

/// Paints every pixel of `frame` that differs from `before` and counts
/// them.
pub fn paint_changed(mut frame: Pixels, before: &Pixels) -> Result<(Pixels, usize)> {
    ensure!(
        frame.width == before.width && frame.height == before.height,
        "The frame before has another size, {}x{} and {}x{}",
        before.width,
        before.height,
        frame.width,
        frame.height
    );
    let mut changed = 0;
    let (pixels, old) = (frame.rgba.as_chunks_mut::<4>().0, before.rgba.as_chunks::<4>().0);
    for (pixel, old) in pixels.iter_mut().zip(old) {
        if pixel != old {
            *pixel = CHANGED;
            changed += 1;
        }
    }
    Ok((frame, changed))
}

#[cfg(test)]
mod test {
    use super::{CHANGED, Pixels, paint_changed};

    fn pixels(colors: &[[u8; 4]]) -> Pixels {
        Pixels {
            rgba:   colors.concat(),
            width:  u32::try_from(colors.len()).unwrap(),
            height: 1,
        }
    }

    #[test]
    fn paints_only_the_changed_pixels() {
        let white = [255; 4];
        let black = [0, 0, 0, 255];
        let before = pixels(&[white, white, black]);
        let frame = pixels(&[white, black, black]);

        let (painted, changed) = paint_changed(frame, &before).unwrap();

        assert_eq!(changed, 1);
        assert_eq!(painted.rgba, [white, CHANGED, black].concat());
    }

    #[test]
    fn refuses_another_size() {
        let white = [255; 4];
        assert!(paint_changed(pixels(&[white]), &pixels(&[white, white])).is_err());
    }
}
