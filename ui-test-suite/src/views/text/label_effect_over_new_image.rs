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

        check_colors("")?;
        Ok(())
    }
}
