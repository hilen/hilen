use anyhow::{Result, ensure};

use crate::{
    self as hilen,
    deps::{hreads::from_main, refs::Weak},
    login::QrCodeView,
    ui::{Label, Setup, ViewData, ViewFrame, ViewTest, view},
    ui_test::check_colors,
};

const LINK: &str = "https://app.example.com/auth/code?code=ABC234";

/// A QR code of a login link, sharp at 2 sizes. A phone has to read the
/// big one off the screen, try it.
#[view]
struct QrCodeViewTest {
    #[init]
    title: Label,
    big:   QrCodeView,
    small: QrCodeView,
    link:  Label,
}

impl Setup for QrCodeViewTest {
    fn setup(self: Weak<Self>) {
        self.title.set_text("a QR code of the link below").set_text_size(20);
        self.title.place().lrt(10).h(40);

        self.big.set_frame((40, 70, 360, 360));
        self.big.set_corner_radius(12);
        self.big.set_text(LINK);

        self.small.set_frame((430, 70, 130, 130));
        self.small.set_text(LINK);

        self.link.set_text(LINK).set_text_size(18);
        self.link.place().lrb(10).h(40);
    }
}

impl ViewTest for QrCodeViewTest {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        let text = from_main(move || view.big.text().to_string());
        ensure!(text == LINK, "the view holds {text}");

        check_colors(CODES)?;

        Ok(())
    }
}

const CODES: &str = "";
