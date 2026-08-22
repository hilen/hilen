use anyhow::Result;
use hilen::{
    refs::Weak,
    ui::{CheckBox, Setup, ViewFrame, ViewTest, view},
    ui_test::inject_touches,
};

#[view]
struct CheckBoxTestView {
    #[init]
    checkbox: CheckBox,
}

impl Setup for CheckBoxTestView {
    fn setup(self: Weak<Self>) {
        self.checkbox.set_frame((50, 50, 50, 50));
    }
}

impl ViewTest for CheckBoxTestView {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        assert!(!view.checkbox.on());

        inject_touches(
            "
         81   86   b
         81   86   e

     ",
        );

        assert!(view.checkbox.on());

        inject_touches(
            "
         81   86   b
         81   86   e

     ",
        );

        assert!(!view.checkbox.on());

        Ok(())
    }
}
