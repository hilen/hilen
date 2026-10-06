use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    refs::{Weak, manage::DataManager},
    system::{Clipboard, ClipboardImage},
    ui::{
        Image, ImageView, Label, ModifiersState, Setup, TextAlignment, TextField, VerticalAlignment,
        ViewData, ViewTest, view,
    },
    ui_test::{checkpoint, inject_keys, inject_modifiers, system_input::wait_until},
};

const COPIED_TEXT: &str = "a text";

const WIDTH: u32 = 120;
const HEIGHT: u32 = 80;

/// A paste into a text field with a picture in the clipboard and no text
/// fires `image_pasted` with that picture. A paste with a text in the
/// clipboard types the text and fires nothing.
#[view]
struct TextFieldImagePaste {
    pasted: Vec<ClipboardImage>,

    #[init]
    field_title:   Label,
    field:         TextField,
    status_title:  Label,
    status:        Label,
    picture_title: Label,
    picture:       ImageView,
}

impl Setup for TextFieldImagePaste {
    fn setup(self: Weak<Self>) {
        let title = |label: Weak<Label>, text: &str, y: f32| {
            label.set_text(text).set_text_size(18);
            label.set_alignment(TextAlignment::Left);
            label.place().t(y).lr(20).h(26);
        };

        title(self.field_title, "text field, paste with Cmd or Ctrl and V", 20.0);
        self.field.set_text_size(20);
        self.field.place().t(50).lr(20).h(44);

        title(self.status_title, "what the image_pasted event gave", 120.0);
        self.status.set_text_size(18).set_multiline(true);
        self.status.set_alignment(TextAlignment::Left);
        self.status.set_vertical_alignment(VerticalAlignment::Top);
        self.status.place().t(150).lr(20).h(60);

        title(self.picture_title, "the pasted picture", 230.0);
        self.picture.place().t(262).l(20).size(WIDTH, HEIGHT);

        self.field.image_pasted.val(move |image| self.image_pasted(image));
        self.show_status();
    }
}

impl TextFieldImagePaste {
    fn image_pasted(mut self: Weak<Self>, image: ClipboardImage) {
        self.picture.set_image(Image::load(&image.png, "text field image paste"));
        self.pasted.push(image);
        self.show_status();
    }

    fn show_status(self: Weak<Self>) {
        let text = match self.pasted.last() {
            Some(image) => format!(
                "fired {} time, a png of {} by {} pixels",
                self.pasted.len(),
                image.width,
                image.height
            ),
            None => "not fired yet".to_string(),
        };
        self.status.set_text(text);
    }

    fn pasted(self: Weak<Self>) -> Vec<ClipboardImage> {
        from_main(move || self.pasted.clone())
    }

    fn field_text(self: Weak<Self>) -> String {
        from_main(move || self.field.text().to_string())
    }
}

/// 4 flat panels, red, green, blue and yellow, so a picture that came
/// back turned or with swapped colors is seen at once.
fn copied_picture() -> Result<ClipboardImage> {
    let mut rgba = Vec::new();
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let pixel: [u8; 4] = match (x < WIDTH / 2, y < HEIGHT / 2) {
                (true, true) => [220, 40, 40, 255],
                (false, true) => [40, 180, 80, 255],
                (true, false) => [40, 90, 220, 255],
                (false, false) => [240, 200, 40, 255],
            };
            rgba.extend_from_slice(&pixel);
        }
    }
    ClipboardImage::from_rgba(&rgba, WIDTH, HEIGHT)
}

fn paste() {
    inject_modifiers(ModifiersState::SUPER);
    inject_keys("v");
    inject_modifiers(ModifiersState::empty());
}

impl ViewTest for TextFieldImagePaste {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        // The clipboard of the user is never read and never replaced,
        // also when the test runs in a window.
        from_main(|| Clipboard::set_in_process(true));

        from_main(|| Clipboard::set_text(COPIED_TEXT))?;
        from_main(move || view.field.focus());
        paste();
        ensure!(
            view.field_text() == COPIED_TEXT,
            "a paste of a text typed {:?}",
            view.field_text()
        );
        ensure!(view.pasted().is_empty(), "a paste of a text fired image_pasted");
        checkpoint("a text in the clipboard, the paste typed it and the event is not fired yet")?;

        let picture = copied_picture()?;
        let copied = picture.clone();
        from_main(move || Clipboard::set_image(&copied))?;
        paste();
        // The png file is made on another thread, the event comes some
        // frames after the keys.
        wait_until("the image_pasted event", move || !view.pasted.is_empty())?;

        let pasted = view.pasted();
        ensure!(pasted.len() == 1, "image_pasted fired {} times", pasted.len());
        ensure!(
            (pasted[0].width, pasted[0].height) == (WIDTH, HEIGHT),
            "the pasted picture is {} by {} pixels",
            pasted[0].width,
            pasted[0].height
        );
        ensure!(pasted[0] == picture, "the pasted png is not the copied one");
        ensure!(
            view.field_text() == COPIED_TEXT,
            "a paste of a picture changed the text to {:?}",
            view.field_text()
        );
        ensure!(
            from_main(move || view.status.text() == "fired 1 time, a png of 120 by 80 pixels"),
            "the status does not show the pasted picture"
        );
        checkpoint(
            "a picture in the clipboard, the event fired 1 time and the picture is shown, the text stays",
        )?;

        Ok(())
    }
}
