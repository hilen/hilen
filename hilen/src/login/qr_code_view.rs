use log::error;
use qrcode::{Color as Module, QrCode};
use ui_proc::view;

use crate::{
    deps::refs::{Weak, manage::DataManager, weak_from_ref},
    gm::{color::WHITE, flat::Size},
    ui::{Image, ImageFilter, ImageMode, ImageView, Setup, ViewData},
};

/// The white frame a reader needs around the code, in modules. The QR
/// standard asks for 4.
const QUIET_ZONE: usize = 4;

/// Draws a text as a QR code, black on white whatever the theme, a phone
/// camera reads nothing else for sure. It fills the view and stays square.
#[view]
pub struct QrCodeView {
    text: String,

    #[init]
    image: ImageView,
}

impl QrCodeView {
    /// The text the code holds, a link most of the time. A text too long
    /// for a QR code is logged and the view stays as it was.
    pub fn set_text(&self, text: impl ToString) -> &Self {
        let mut this = weak_from_ref(self);
        let text = text.to_string();
        if text == this.text {
            return self;
        }

        let Some((pixels, side)) = pixels(&text) else {
            error!("no QR code fits a text of {} bytes", text.len());
            return self;
        };

        // One pixel per module, drawn with no blur between the modules. A
        // picture stays in memory under its name for good, and this one is
        // a few KB.
        let name = format!("qr-code:{text}");
        let image = Image::get_existing(&name).unwrap_or_else(|| {
            let mut image = Image::from_raw_data(pixels, name, Size::new(side, side), 4);
            image.set_filter(ImageFilter::Nearest);
            image
        });

        this.image.set_image(image);
        this.text = text;
        self
    }

    pub fn text(&self) -> &str {
        &self.text
    }
}

impl Setup for QrCodeView {
    fn setup(mut self: Weak<Self>) {
        self.set_color(WHITE);
        self.image.mode = ImageMode::AspectFit;
        self.image.place().back();
    }
}

/// RGBA pixels of the code, one per module, with the quiet zone around it,
/// and the side of the square. None when the text does not fit a code.
fn pixels(text: &str) -> Option<(Vec<u8>, u32)> {
    let code = QrCode::new(text.as_bytes()).ok()?;
    let width = code.width();
    let side = width + QUIET_ZONE * 2;

    let mut pixels = vec![255; side * side * 4];
    for (index, module) in code.to_colors().into_iter().enumerate() {
        if module == Module::Dark {
            let (x, y) = (index % width + QUIET_ZONE, index / width + QUIET_ZONE);
            let at = (y * side + x) * 4;
            pixels[at..at + 3].fill(0);
        }
    }

    Some((pixels, u32::try_from(side).ok()?))
}

#[cfg(test)]
mod test {
    use super::{QUIET_ZONE, pixels};

    fn dark(pixels: &[u8], side: u32, x: usize, y: usize) -> bool {
        let side = usize::try_from(side).expect("a side fits usize");
        pixels[(y * side + x) * 4] == 0
    }

    #[test]
    fn a_code_has_its_finder_corner_and_a_white_frame() {
        let (pixels, side) = pixels("https://app.example.com/auth/code?code=ABC234").expect("the link fits");
        let edge = usize::try_from(side).expect("a side fits usize");
        assert_eq!(pixels.len(), edge * edge * 4);

        // The quiet zone is white all around.
        for at in 0..edge {
            for inset in 0..QUIET_ZONE {
                assert!(!dark(&pixels, side, at, inset));
                assert!(!dark(&pixels, side, inset, at));
                assert!(!dark(&pixels, side, at, edge - 1 - inset));
                assert!(!dark(&pixels, side, edge - 1 - inset, at));
            }
        }

        // Every QR code starts with a finder pattern in the top left: a dark
        // ring of 7 modules, a white ring inside it, a dark 3 by 3 core.
        let q = QUIET_ZONE;
        assert!(dark(&pixels, side, q, q));
        assert!(dark(&pixels, side, q + 6, q));
        assert!(!dark(&pixels, side, q + 1, q + 1));
        assert!(dark(&pixels, side, q + 3, q + 3));
        // And the white separator next to it.
        assert!(!dark(&pixels, side, q + 7, q));

        // Opaque everywhere.
        assert!(pixels.as_chunks::<4>().0.iter().all(|pixel| pixel[3] == 255));
    }

    #[test]
    fn a_text_too_long_for_a_code_gives_none() {
        assert!(pixels(&"x".repeat(8000)).is_none());
    }
}
