use anyhow::Result;
use hilen::{
    dispatch::{from_main, on_main},
    inspect::{ViewToInspect, protocol::ui::ViewRepr},
    refs::Weak,
    ui::{BLUE, Button, Setup, ViewData, ViewTest, view},
};
use pretty_assertions::assert_eq;

#[view]
struct InspectParsing {
    #[init]
    button: Button,
}

impl Setup for InspectParsing {
    fn setup(self: Weak<Self>) {
        self.button.place().t(20).l(20).size(100, 100);
        self.button.set_color(BLUE);
    }
}

impl ViewTest for InspectParsing {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        let repr = from_main(move || view.view_to_inspect());

        let json = serde_json::to_string(&repr)?;

        let parsed_repr: ViewRepr = serde_json::from_str(&json)?;

        assert_eq!(repr, parsed_repr);

        on_main(move || {
            drop(parsed_repr);
            drop(repr);
        });

        Ok(())
    }
}
