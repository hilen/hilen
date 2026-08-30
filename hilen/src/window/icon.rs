//! The process icon from encoded image bytes. macOS draws the Dock image
//! as is, so the iOS style square gets the Dock shape here first.

use anyhow::Result;
#[cfg(macos)]
use anyhow::anyhow;

#[cfg(any(win, linux))]
use crate::window::Window;

#[cfg(macos)]
pub(super) fn apply_icon(data: &[u8]) -> Result<()> {
    use objc2::AllocAnyThread;
    use objc2_app_kit::{NSApplication, NSImage};
    use objc2_foundation::{MainThreadMarker, NSData};

    let mtm = MainThreadMarker::new().ok_or_else(|| anyhow!("not on the main thread"))?;
    let data = mac_icon(data)?;
    let image = NSImage::initWithData(NSImage::alloc(), &NSData::with_bytes(&data))
        .ok_or_else(|| anyhow!("not a decodable image"))?;
    unsafe {
        NSApplication::sharedApplication(mtm).setApplicationIconImage(Some(&image));
    }
    Ok(())
}

#[cfg(any(win, linux))]
pub(super) fn apply_icon(data: &[u8]) -> Result<()> {
    use winit::window::Icon;

    let image = image::load_from_memory(data)?.into_rgba8();
    let (width, height) = image.dimensions();
    let icon = Icon::from_rgba(image.into_raw(), width, height)?;
    if let Some(window) = Window::winit_window() {
        window.set_window_icon(Some(icon));
    }
    Ok(())
}

#[cfg(not(desktop))]
pub(super) fn apply_icon(_: &[u8]) -> Result<()> {
    Ok(())
}

/// A square iOS style icon reshaped into the macOS Dock shape. The Dock
/// draws the image as is, so the rounded corners and the transparent
/// margin around them have to be in the pixels. The proportions follow
/// the Apple icon template, an 824 point rounded square on a 1024 canvas.
#[cfg(macos)]
fn mac_icon(data: &[u8]) -> Result<Vec<u8>> {
    use std::io::Cursor;

    use image::{ImageFormat, RgbaImage, imageops::FilterType};

    use crate::gm::LossyConvert;

    const CANVAS: u32 = 1024;
    const ICON: u32 = 824;
    const RADIUS: f32 = 185.4;

    let source = image::load_from_memory(data)?
        .resize_exact(ICON, ICON, FilterType::Lanczos3)
        .into_rgba8();

    let margin = (CANVAS - ICON) / 2;
    let mut canvas = RgbaImage::new(CANVAS, CANVAS);
    for (x, y, pixel) in source.enumerate_pixels() {
        let mut pixel = *pixel;
        let coverage = rounded_corner_coverage(x, y, ICON, RADIUS);
        pixel.0[3] = (f32::from(pixel.0[3]) * coverage).round().lossy_convert();
        canvas.put_pixel(x + margin, y + margin, pixel);
    }

    let mut png = Vec::new();
    canvas.write_to(&mut Cursor::new(&mut png), ImageFormat::Png)?;
    Ok(png)
}

/// How much of a pixel lies inside the rounded square, 1 inside, 0 outside
/// and a fraction on the curve so the edge is not jagged.
#[cfg(macos)]
fn rounded_corner_coverage(x: u32, y: u32, size: u32, radius: f32) -> f32 {
    use crate::gm::LossyConvert;

    let size: f32 = size.lossy_convert();
    let px: f32 = x.lossy_convert();
    let py: f32 = y.lossy_convert();
    let px = px + 0.5;
    let py = py + 0.5;
    let dx = (radius - px).max(px - (size - radius)).max(0.0);
    let dy = (radius - py).max(py - (size - radius)).max(0.0);
    let distance = (dx * dx + dy * dy).sqrt();
    (radius + 0.5 - distance).clamp(0.0, 1.0)
}

#[cfg(all(test, macos))]
mod test {
    use anyhow::Result;

    use super::mac_icon;

    #[test]
    fn dock_shape_has_transparent_corners_and_margin() -> Result<()> {
        let mut png = Vec::new();
        image::RgbaImage::from_pixel(64, 64, image::Rgba([255, 0, 0, 255]))
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)?;

        let icon = image::load_from_memory(&mac_icon(&png)?)?.into_rgba8();

        assert_eq!(icon.dimensions(), (1024, 1024));
        assert_eq!(icon.get_pixel(0, 0).0[3], 0, "margin is transparent");
        assert_eq!(icon.get_pixel(100, 100).0[3], 0, "corner is cut");
        assert_eq!(
            icon.get_pixel(512, 512).0,
            [255, 0, 0, 255],
            "center is the source"
        );
        assert_eq!(icon.get_pixel(512, 100).0[3], 255, "edge midpoint is solid");
        Ok(())
    }
}
