use anyhow::Result;
use hilen::{
    AppRunner,
    dispatch::from_main,
    refs::Weak,
    ui::{Label, Screenshot, Setup, U8Color, ViewFrame, ViewTest, view},
};

const TEXT: &str = "Grumpy wizards 123";

const PLAIN_FRAME: (u32, u32, u32, u32) = (20, 20, 400, 80);
const SPACED_FRAME: (u32, u32, u32, u32) = (20, 120, 400, 80);

#[view]
struct LetterSpacing {
    #[init]
    plain:  Label,
    spaced: Label,
}

impl Setup for LetterSpacing {
    fn setup(self: Weak<Self>) {
        self.plain.set_frame(PLAIN_FRAME);
        self.plain.set_text(TEXT).set_text_size(30);

        self.spaced.set_frame(SPACED_FRAME);
        self.spaced.set_text(TEXT).set_text_size(30);
    }
}

impl ViewTest for LetterSpacing {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        let original = AppRunner::take_screenshot()?;

        assert!(
            region(&original, SPACED_FRAME) == shifted_region(&original, PLAIN_FRAME, 100),
            "labels with equal settings render differently"
        );

        from_main(move || {
            view.spaced.set_letter_spacing(5);
        });

        let spaced = AppRunner::take_screenshot()?;

        assert!(
            region(&spaced, SPACED_FRAME) != region(&original, SPACED_FRAME),
            "positive letter spacing did not change the rendering"
        );
        assert!(
            region(&spaced, PLAIN_FRAME) == region(&original, PLAIN_FRAME),
            "letter spacing on one label changed another label"
        );

        from_main(move || {
            view.spaced.set_letter_spacing(-2);
        });

        let tightened = AppRunner::take_screenshot()?;

        assert!(
            region(&tightened, SPACED_FRAME) != region(&spaced, SPACED_FRAME),
            "negative letter spacing did not change the rendering"
        );

        from_main(move || {
            view.spaced.set_letter_spacing(0);
        });

        let restored = AppRunner::take_screenshot()?;

        assert!(
            region(&restored, SPACED_FRAME) == region(&original, SPACED_FRAME),
            "zero letter spacing did not restore the original rendering"
        );

        Ok(())
    }
}

fn region(shot: &Screenshot, frame: (u32, u32, u32, u32)) -> Vec<U8Color> {
    shifted_region(shot, frame, 0)
}

fn shifted_region(shot: &Screenshot, frame: (u32, u32, u32, u32), shift_y: u32) -> Vec<U8Color> {
    let (x, y, width, height) = frame;
    let mut pixels = Vec::with_capacity((width * height) as usize);

    for py in y + shift_y..y + shift_y + height {
        for px in x..x + width {
            pixels.push(shot.get_pixel((px, py)));
        }
    }

    pixels
}
