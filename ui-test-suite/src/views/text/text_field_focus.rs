use anyhow::Result;
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{Setup, TextField, ViewData, ViewTest, view},
    ui_test::inject_keys,
};

/// `TextField::focus` must start the same editing session a tap starts,
/// so typed keys land in the field without any touch. The kukareker
/// project palette opens from a shortcut and types straight away.
#[view]
struct TextFieldFocus {
    #[init]
    field: TextField,
    other: TextField,
}

impl Setup for TextFieldFocus {
    fn setup(self: Weak<Self>) {
        self.field.place().tl(20).size(300, 40);
        self.other.place().tl(80).size(300, 40);
    }
}

impl ViewTest for TextFieldFocus {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        from_main(move || view.field.focus());
        inject_keys("abc");
        assert_eq!(from_main(move || view.field.text().to_string()), "abc");
        assert_eq!(from_main(move || view.other.text().to_string()), "");

        // Focus moves between fields, the first one ends its session.
        from_main(move || view.other.focus());
        inject_keys("xy");
        assert_eq!(from_main(move || view.field.text().to_string()), "abc");
        assert_eq!(from_main(move || view.other.text().to_string()), "xy");

        // Refocusing puts the caret at the end, typing appends.
        from_main(move || view.field.focus());
        inject_keys("d");
        assert_eq!(from_main(move || view.field.text().to_string()), "abcd");

        Ok(())
    }
}
