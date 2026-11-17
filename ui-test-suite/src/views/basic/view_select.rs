use anyhow::Result;
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{
        Container, GREEN, LIGHT_GRAY, Label, Setup, UIManager, ViewData, ViewFrame, ViewTest, ViewTouch, view,
    },
    ui_test::checkpoint,
};

/// A view becomes the selected one with no press on it, and the view
/// that was selected is so no more. A box is green while it is selected.
#[view]
struct ViewSelect {
    #[init]
    first_title:  Label,
    first:        Container,
    second_title: Label,
    second:       Container,
    state:        Label,
}

impl ViewSelect {
    fn paint(self: Weak<Self>) {
        let color = |selected: bool| if selected { GREEN } else { LIGHT_GRAY };
        self.first.set_color(color(self.first.is_selected()));
        self.second.set_color(color(self.second.is_selected()));
        self.state
            .set_text(match (self.first.is_selected(), self.second.is_selected()) {
                (true, false) => "the first box is selected",
                (false, true) => "the second box is selected",
                (false, false) => "no box is selected",
                (true, true) => "both boxes are selected",
            });
    }
}

impl Setup for ViewSelect {
    fn setup(self: Weak<Self>) {
        self.first_title.set_frame((20, 20, 560, 24));
        self.first_title.set_text("first box, green while it is selected");
        self.first.set_frame((20, 50, 560, 120));

        self.second_title.set_frame((20, 200, 560, 24));
        self.second_title.set_text("second box, green while it is selected");
        self.second.set_frame((20, 230, 560, 120));

        self.state.set_frame((20, 380, 560, 30));
        self.paint();
    }
}

impl ViewTest for ViewSelect {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        from_main(move || {
            assert!(!view.first.is_selected() && !view.second.is_selected());
        });
        checkpoint("no box is selected, both are gray")?;

        from_main(move || {
            view.first.select();
            assert!(
                view.first.is_selected(),
                "the first box is not selected after select"
            );
            view.paint();
        });
        checkpoint("the first box is selected and green")?;

        from_main(move || {
            view.second.select();
            assert!(
                view.second.is_selected(),
                "the second box is not selected after select"
            );
            assert!(!view.first.is_selected(), "the first box is still selected");
            view.paint();
        });
        checkpoint("the second box is selected, the first is gray again")?;

        from_main(move || {
            UIManager::unselect_view();
            assert!(!view.second.is_selected(), "the second box is still selected");
            view.paint();
        });
        checkpoint("no box is selected, both are gray")?;

        Ok(())
    }
}
