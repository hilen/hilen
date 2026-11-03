//! Pictures of the screens of the system, see `docs/screen-capture.md`.
//! macOS has it through [xcap](https://github.com/nashaofu/xcap), the other
//! systems answer with an error for now.

#[cfg(test)]
mod tests;

use std::io::Cursor;

use anyhow::{Context, Result, bail, ensure};
use image::{ImageEncoder, RgbaImage, codecs::png::PngEncoder, imageops::crop_imm};

use crate::{
    deps::refs::{Weak, manage::DataManager},
    gm::{
        LossyConvert,
        flat::{Rect, Size},
    },
    window::{Display, image::Image},
};

/// The Screen Recording permission of macOS. Without it a picture of a
/// screen shows only the desktop and the windows of this app.
pub struct ScreenCapture;

impl ScreenCapture {
    /// The user allowed this app to read the screens.
    pub fn allowed() -> bool {
        #[cfg(macos)]
        return objc2_core_graphics::CGPreflightScreenCaptureAccess();
        #[cfg(not(macos))]
        false
    }

    /// Asks the user for the permission. The system shows its dialog only
    /// the first time and answers the old choice after that. A new grant
    /// counts only after the app starts again.
    pub fn ask() -> bool {
        #[cfg(macos)]
        return objc2_core_graphics::CGRequestScreenCaptureAccess();
        #[cfg(not(macos))]
        false
    }
}

/// A picture of 1 screen, taken at its full pixel size.
pub struct ScreenPicture {
    /// The screen, in the points of `Display::all`.
    pub display: Display,
    pub size:    Size<u32>,
    rgba:        Vec<u8>,
}

impl ScreenPicture {
    /// A picture of every screen, the main one first. It takes some tens of
    /// ms for each screen, so it runs off the main thread too.
    pub fn all() -> Result<Vec<Self>> {
        #[cfg(macos)]
        {
            let mut pictures = Vec::new();
            for monitor in xcap::Monitor::all().context("The screens could not be listed")? {
                pictures.push(Self::of(&monitor)?);
            }
            pictures.sort_by_key(|(main, _)| !*main);
            Ok(pictures.into_iter().map(|(_, picture)| picture).collect())
        }
        #[cfg(not(macos))]
        bail!("Screen pictures are on macOS only for now, see docs/screen-capture.md")
    }

    #[cfg(macos)]
    fn of(monitor: &xcap::Monitor) -> Result<(bool, Self)> {
        let name = monitor.name().ok();
        let image = monitor
            .capture_image()
            .with_context(|| format!("No picture of the screen {name:?}"))?;
        let (x, y, width, height): (f32, f32, f32, f32) = (
            monitor.x()?.lossy_convert(),
            monitor.y()?.lossy_convert(),
            monitor.width()?.lossy_convert(),
            monitor.height()?.lossy_convert(),
        );
        let picture = Self::new(
            Display {
                name,
                frame: (x, y, width, height).into(),
                scale: monitor.scale_factor()?,
            },
            image,
        )?;
        Ok((monitor.is_primary()?, picture))
    }

    /// A picture of `display` from RGBA pixels.
    pub fn new(display: Display, image: RgbaImage) -> Result<Self> {
        ensure!(
            image.width() > 0 && image.height() > 0,
            "The picture of {:?} is empty",
            display.name
        );
        Ok(Self {
            display,
            size: Size::new(image.width(), image.height()),
            rgba: image.into_raw(),
        })
    }

    /// The pixels in 1 point, the picture can be taken at another scale
    /// than the screen reports.
    fn pixels_per_point(&self) -> f32 {
        let width: f32 = self.size.width.lossy_convert();
        width / self.display.frame.size.width
    }

    /// The picture as an image to show, stored under `name`. On the main
    /// thread.
    pub fn image(&self, name: &str) -> Weak<Image> {
        Image::free_with_name(name);
        Image::from_raw_data(self.rgba.clone(), name, self.size, 4)
    }

    /// The part of the picture under `rect` as a png. `rect` is in points
    /// from the top left of this screen.
    pub fn png_of(&self, rect: Rect) -> Result<Vec<u8>> {
        let scale = self.pixels_per_point();
        let left: u32 = (rect.origin.x * scale).round().max(0.0).lossy_convert();
        let top: u32 = (rect.origin.y * scale).round().max(0.0).lossy_convert();
        let right: u32 = ((rect.origin.x + rect.size.width) * scale).round().max(0.0).lossy_convert();
        let bottom: u32 = ((rect.origin.y + rect.size.height) * scale).round().max(0.0).lossy_convert();
        let right = right.min(self.size.width);
        let bottom = bottom.min(self.size.height);
        if right <= left || bottom <= top {
            bail!("{rect:?} is not on the screen of {:?}", self.display.name);
        }

        let whole = RgbaImage::from_raw(self.size.width, self.size.height, self.rgba.clone())
            .context("The pixels do not fill the picture")?;
        let part = crop_imm(&whole, left, top, right - left, bottom - top).to_image();

        let mut png = Vec::new();
        PngEncoder::new(Cursor::new(&mut png)).write_image(
            part.as_raw(),
            part.width(),
            part.height(),
            image::ExtendedColorType::Rgba8,
        )?;
        Ok(png)
    }
}
