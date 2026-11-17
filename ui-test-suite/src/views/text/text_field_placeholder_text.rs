use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{Setup, TextField, ViewData, ViewTest, view},
};

/// `text` is what the user entered. It used to return the placeholder while
/// the field was empty, so an app that read an empty field got the hint back
/// as input. Blackforge parsed "version, empty for the newest" as a version.
#[view]
struct TextFieldPlaceholderText {
    #[init]
    hinted: TextField,
    late:   TextField,
}

impl Setup for TextFieldPlaceholderText {
    fn setup(self: Weak<Self>) {
        self.hinted.set_placeholder("Search");
        self.hinted.set_text_size(32);
        self.hinted.place().tl(20).size(300, 44);

        self.late.set_text_size(32);
        self.late.place().t(84).l(20).size(300, 44);
    }
}

struct Read {
    text:         String,
    empty:        bool,
    placeholding: bool,
}

fn read(field: Weak<TextField>) -> Read {
    from_main(move || Read {
        text:         field.text().to_string(),
        empty:        field.is_empty(),
        placeholding: field.is_placeholding(),
    })
}

impl ViewTest for TextFieldPlaceholderText {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        let shown = read(view.hinted);
        ensure!(shown.placeholding, "an empty field shows its placeholder");
        ensure!(
            shown.text.is_empty(),
            "the placeholder came back as text: {:?}",
            shown.text
        );
        ensure!(shown.empty, "a field that only shows its placeholder is empty");

        from_main(move || {
            view.hinted.set_text("Zug");
        });
        let entered = read(view.hinted);
        ensure!(
            entered.text == "Zug",
            "entered text reads back, got {:?}",
            entered.text
        );
        ensure!(!entered.empty && !entered.placeholding);

        from_main(move || {
            view.hinted.clear();
        });
        let cleared = read(view.hinted);
        ensure!(
            cleared.placeholding,
            "a cleared field shows its placeholder again"
        );
        ensure!(
            cleared.text.is_empty(),
            "the placeholder came back after clear: {:?}",
            cleared.text
        );
        ensure!(cleared.empty);

        // A placeholder given to a field that is already empty.
        from_main(move || {
            view.late.set_placeholder("Later");
        });
        let late = read(view.late);
        ensure!(
            late.text.is_empty(),
            "the late placeholder came back as text: {:?}",
            late.text
        );
        ensure!(late.empty);

        Ok(())
    }
}
