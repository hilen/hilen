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

/// `replace_range` swaps a part of the text for another, the way dictation
/// swaps its first guess of a sentence for a better one. The caret ends after
/// the new part, so a key typed next follows it.
#[view]
struct TextFieldReplaceRange {
    #[init]
    title: Label,
    field: TextField,
}

impl Setup for TextFieldReplaceRange {
    fn setup(mut self: Weak<Self>) {
        self.title.set_text("field, a part of the text is replaced").set_text_size(18);
        self.title.set_alignment(TextAlignment::Left);
        self.title.place().t(60).lr(20).h(26);

        self.field.set_text_size(24);
        self.field.set_alignment(TextAlignment::Left);
        self.field.place().t(90).lr(20).h(50);
    }
}

impl ViewTest for TextFieldReplaceRange {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        tap(FIELD.0, FIELD.1)?;
        wait_until("the field is edited", move || view.field.is_editing())?;
        type_text("say hello wold")?;
        wait_until("the field holds the typed text", move || {
            view.field.text() == "say hello wold"
        })?;

        from_main(move || {
            ensure!(view.field.caret() == 14, "the caret is at {}", view.field.caret());
            view.field.replace_range(4..14, "hello world");
            ensure!(view.field.caret() == 15, "the caret is at {}", view.field.caret());
            Ok(())
        })?;
        type_text("!")?;
        wait_until("the typed key follows the new part", move || {
            view.field.text() == "say hello world!"
        })?;
        checkpoint("the field holds the replaced words and the key typed after them")?;

        from_main(move || {
            view.field.set_text("žodis ab");
            view.field.replace_range(1..2, "Ž");
            ensure!(
                view.field.text() == "Žodis ab",
                "the field holds {:?}",
                view.field.text()
            );
            view.field.replace_range(7..100, "cd");
            ensure!(
                view.field.text() == "Žodis cd",
                "the field holds {:?}",
                view.field.text()
            );
            ensure!(view.field.caret() == 9, "the caret is at {}", view.field.caret());
            Ok(())
        })?;
        checkpoint("an edge inside a char and an end past the text")
    }
}
