use anyhow::Result;
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{Color, GRAY, Image, ImageView, Label, Setup, ViewData, ViewFrame, ViewTest, WHITE, view},
    ui_test::{check_colors, checkpoint},
};

const SIDE: u32 = 8;

fn orange(name: &str) -> Weak<Image> {
    let pixels = [255, 140, 0, 255].repeat((SIDE * SIDE) as usize);
    Image::from_raw_data(pixels, name, (SIDE, SIDE).into(), 4)
}

const CHECK_1: &str = r"
     480   44 - #000000
     244   48 - #000000
     516   48 - #000000
      80   52 - #1a242b
     136   52 - #435d70
     204   52 - #435d70
     400   52 - #1d2830
      24   80 - #bcbcbc
     320  120 - #ff8c00
     116  136 - #cca590
     160  148 - #d8812b
     508  152 - #c17b45
     380  156 - #fffeff
     124  160 - #ece6e8
     224  164 - #a37366
      64  176 - #ffffff
     352  176 - #a87461
     448  176 - #ff8c00
     384  184 - #9a7070
     168  192 - #ffffff
     532  192 - #df8324
     120  200 - #f0e4dc
     492  204 - #e08323
      80  220 - #b27756
     372  220 - #af7659
     316  304 - #4a677b
     344  304 - #25343e
     368  304 - #131b20
     444  308 - #597c95
     180  312 - #597c95
     260  312 - #2d3f4c
     592  592 - #597c95
";

/// 2 picture views side by side, each with the same white text with a blue
/// glow over it and a title that says when its picture is set. The left
/// picture is there from the start. The right box is empty at first, so its
/// glow image is made and drawn before its picture exists, then it gets a
/// picture the process has never drawn. Proves the right side then looks
/// like the left one. Image batches once drew in the order their images
/// were first drawn, so the older glow went under the new picture and cut
/// holes into it.
#[view]
struct LabelEffectOverNewImage {
    #[init]
    early_title: Label,
    early:       ImageView,
    early_text:  Label,
    late_title:  Label,
    late:        ImageView,
    late_text:   Label,
    note:        Label,
}

impl Setup for LabelEffectOverNewImage {
    fn setup(self: Weak<Self>) {
        let columns = [
            (
                self.early_title,
                self.early,
                self.early_text,
                20,
                "picture set at the start",
            ),
            (
                self.late_title,
                self.late,
                self.late_text,
                310,
                "picture set later",
            ),
        ];
        for (title, picture, text, left, name) in columns {
            title.set_frame((left, 30, 270, 40));
            title.set_text(name).set_text_size(22);

            picture.set_frame((left, 80, 270, 180));
            picture.set_border_color(GRAY).set_border_width(2);

            text.set_frame((left, 80, 270, 180));
            text.set_text("glow").set_text_size(90).set_text_color(WHITE);
            text.set_text_shadow(Color::rgb(0.1, 0.3, 1.0), (0, 0)).set_text_shadow_blur(10);
        }
        self.early.set_image(orange("label effect early picture"));

        self.note.set_frame((20, 290, 560, 40));
        self.note.set_text("the right box has no picture yet").set_text_size(22);
    }
}

impl ViewTest for LabelEffectOverNewImage {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        checkpoint("left has its picture, the right box is still empty")?;

        from_main(move || {
            view.late.set_image(orange("label effect late picture"));
            view.note.set_text("both sides must look the same");
        });

        check_colors(CHECK_1)?;
        Ok(())
    }
}
