use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{Label, Setup, TextAlignment, TextField, ViewData, ViewTest, view},
    ui_test::{
        checkpoint,
        system_input::{tap, type_text, wait_until},
    },
};

const FIELD: (f32, f32) = (540.0, 115.0);
const OUTSIDE: (f32, f32) = (300.0, 540.0);
const PATH: &str = "/tmp/clip.mp4 ";

/// `insert_text` puts a text in at the caret, the way a dropped file puts its
/// path into a chat box. The caret ends after it, so a key typed next follows
/// the path. A field that is not edited takes it at its last caret.
#[view]
struct TextFieldInsert {
    changes: Vec<String>,

    #[init]
    title: Label,
    field: TextField,
    log:   Label,
}

impl Setup for TextFieldInsert {
    fn setup(mut self: Weak<Self>) {
        self.title.set_text("field, a path goes in at the caret").set_text_size(18);
        self.title.set_alignment(TextAlignment::Left);
        self.title.place().t(60).lr(20).h(26);

        self.field.set_text_size(24);
        self.field.set_alignment(TextAlignment::Left);
        self.field.place().t(90).lr(20).h(50);
        self.field.changed.val(move |text| self.note_change(text));

        self.log.set_text_size(18).set_multiline(true);
        self.log.set_alignment(TextAlignment::Left);
        self.log.place().t(160).lr(20).h(120);
        self.log.set_text("no change yet");
    }
}

impl TextFieldInsert {
    fn note_change(mut self: Weak<Self>, text: String) {
        self.changes.push(text);
        self.log.set_text(format!(
            "changed {} times, last to: {}",
            self.changes.len(),
            self.changes.last().map_or("", String::as_str)
        ));
    }
}

impl ViewTest for TextFieldInsert {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        tap(FIELD.0, FIELD.1)?;
        wait_until("the field is edited", move || view.field.is_editing())?;
        type_text("ab")?;
        wait_until("the field holds ab", move || view.field.text() == "ab")?;

        from_main(move || view.field.insert_text(PATH));
        type_text("c")?;
        wait_until("c follows the inserted path", move || {
            view.field.text() == "ab/tmp/clip.mp4 c"
        })?;
        checkpoint("the field holds ab, the path, then the c typed after it")?;

        tap(OUTSIDE.0, OUTSIDE.1)?;
        wait_until("the field is not edited", move || !view.field.is_editing())?;
        from_main(move || view.field.insert_text("/tmp/two"));
        checkpoint("a field that is not edited took the second path at its end")?;
        from_main(move || {
            let text = view.field.text();
            ensure!(text == "ab/tmp/clip.mp4 c/tmp/two", "the field holds {text:?}");
            ensure!(
                view.changes.last().map(String::as_str) == Some(text),
                "the last change was {:?}, not the text of the field",
                view.changes.last()
            );
            ensure!(!view.field.is_editing(), "an insert started the editing");
            Ok(())
        })
    }
}
