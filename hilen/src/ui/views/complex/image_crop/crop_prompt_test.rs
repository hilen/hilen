use anyhow::{Result, ensure};

use super::{super::test_photo::quarters_png, ImageCropPrompt, ImageCropRequest};
use crate::{
    self as hilen,
    deps::{hreads::from_main, refs::Weak},
    ui::{Label, ModalView, NamedKey, Setup, ViewData, ViewTest, view},
    ui_test::{check_colors, inject_named_key, inject_touches, system_input::wait_until},
};

/// What the prompt gave back.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum Outcome {
    #[default]
    Open,
    /// Save, with the size of the png.
    Saved(u32, u32),
    Cancelled,
}

/// The prompt over a canvas of 600: 420 wide and 546 high, so its crop view
/// is 388 square, the zoom row is at 491 and the buttons at 548.
#[view]
struct ImageCropPromptTest {
    result: Outcome,

    #[init]
    status: Label,
}

impl Setup for ImageCropPromptTest {
    fn setup(self: Weak<Self>) {
        self.status.set_text("the page under the prompt").set_text_size(20);
        self.status.place().lrt(0).h(44);
    }
}

impl ImageCropPromptTest {
    fn open(self: Weak<Self>) -> Result<Weak<ImageCropPrompt>> {
        let prompt = from_main(move || {
            let mut this = self;
            this.result = Outcome::Open;
            let prompt =
                ImageCropPrompt::prepare_modally_with_input(ImageCropRequest::new(quarters_png(), 64));
            prompt.modal_event().val(move |png: Option<Vec<u8>>| {
                let mut this = self;
                this.result = match png {
                    Some(png) => image::load_from_memory(&png).map_or(Outcome::Saved(0, 0), |cut| {
                        Outcome::Saved(cut.width(), cut.height())
                    }),
                    None => Outcome::Cancelled,
                };
            });
            prompt
        });
        wait_until("the photo is in the prompt", move || prompt.crop.has_photo())?;
        Ok(prompt)
    }

    fn result(self: Weak<Self>) -> Outcome {
        from_main(move || self.result)
    }
}

impl ViewTest for ImageCropPromptTest {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        let prompt = view.open()?;
        check_colors(CHECK_1)?;

        // The + of the zoom row zooms, the slider follows.
        inject_touches("472 491 b\n472 491 e");
        let (zoom, slider) = from_main(move || (prompt.crop.zoom(), prompt.zoom.value()));
        ensure!((zoom - 1.25).abs() < 0.01, "zoom {zoom} after +");
        ensure!(
            (slider - zoom).abs() < 0.01,
            "the slider is at {slider}, the zoom is {zoom}"
        );
        check_colors(CHECK_2)?;

        // Save gives the png of 64.
        inject_touches("405 548 b\n405 548 e");
        wait_until("Save gave its result", move || view.result != Outcome::Open)?;
        assert_eq!(view.result(), Outcome::Saved(64, 64));

        // Escape gives nothing.
        view.open()?;
        inject_named_key(NamedKey::Escape);
        wait_until("Escape gave its result", move || view.result != Outcome::Open)?;
        assert_eq!(view.result(), Outcome::Cancelled);

        Ok(())
    }
}

const CHECK_1: &str = r"
     272   20 - #080b0d
     384   20 - #000000
     184   24 - #11181d
     232   24 - #354a59
     336   24 - #263540
     292   48 - #9f9fa0
     256   56 - #575759
     296   56 - #737374
     324   56 - #89898a
     592   80 - #354a59
     180  112 - #651616
      92  120 - #f9f9f9
     220  188 - #e03131
     188  364 - #2f9e44
     304  420 - #f59f00
     472  488 - #007aff
     392  544 - #f9f9f9
     212  548 - #7ab8fc
";

const CHECK_2: &str = r"
     272   20 - #080b0d
     384   20 - #000000
     184   24 - #11181d
     232   24 - #354a59
     336   24 - #263540
     292   48 - #9f9fa0
     344   52 - #d7d7d8
     256   56 - #575759
     296   56 - #737374
     324   56 - #89898a
     592   80 - #354a59
     464  128 - #0d3961
     208  348 - #2f9e44
      92  392 - #f9f9f9
     308  420 - #f59f00
     472  488 - #007aff
     392  544 - #f9f9f9
     212  548 - #7ab8fc
";
