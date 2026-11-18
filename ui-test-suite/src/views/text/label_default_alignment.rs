use anyhow::Result;
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{Button, GREEN, Label, Setup, TextAlignment, TextField, ViewData, ViewTest, YELLOW, view},
    ui_test::check_colors,
};

const COLORS: &str = r"
              592    4 - #597c95
              228   40 - #00ff00
              352   52 - #00ff00
               72   64 - #000000
              108   68 - #00ff00
               60   76 - #000000
               88   76 - #00ff00
              248  160 - #ffff00
              496  164 - #ffff00
               76  188 - #ffff00
               56  196 - #000000
              104  196 - #ffff00
              296  304 - #000000
              260  312 - #00ff00
              316  312 - #00ff00
              332  312 - #000100
              296  316 - #00ff00
              120  336 - #00ff00
              556  336 - #00ff00
              296  424 - #000000
              308  428 - #ffff00
              336  428 - #ffff00
              352  428 - #ffff00
              240  432 - #010100
              268  432 - #000000
              320  436 - #010100
               40  456 - #ffff00
              488  456 - #ffff00
                4  592 - #597c95
              160  592 - #597c95
              416  592 - #597c95
              592  592 - #597c95
            ";

/// Text with no alignment set starts at the left, at the text inset. A
/// `Button` and a label set to `Center` keep their text in the middle.
#[view]
struct LabelDefaultAlignment {
    #[init]
    plain:    Label,
    field:    TextField,
    button:   Button,
    centered: Label,
}

impl Setup for LabelDefaultAlignment {
    fn setup(self: Weak<Self>) {
        self.plain.set_color(GREEN);
        self.plain.set_text("label").set_text_size(32);
        self.plain.place().t(40).lr(40).h(60);

        self.field.set_color(YELLOW);
        self.field.set_text("field").set_text_size(32);
        self.field.place().t(160).lr(40).h(60);

        self.button.set_color(GREEN);
        self.button.set_text("button").set_text_size(32);
        self.button.place().t(280).lr(40).h(60);

        self.centered.set_color(YELLOW);
        self.centered.set_alignment(TextAlignment::Center);
        self.centered.set_text("centered").set_text_size(32);
        self.centered.place().t(400).lr(40).h(60);
    }
}

impl ViewTest for LabelDefaultAlignment {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        let (plain_left, plain_inset, centered_left) = from_main(move || {
            (
                !view.plain.alignment.center(),
                view.plain.text_inset(),
                !view.centered.alignment.center(),
            )
        });
        assert!(plain_left, "a label with no alignment set is not left aligned");
        assert!(
            plain_inset > 0.0,
            "a label with no alignment set has no text inset"
        );
        assert!(!centered_left, "an explicit Center did not hold");

        check_colors(COLORS)?;

        Ok(())
    }
}
