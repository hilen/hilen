use anyhow::Result;
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{
        Alert, Button, ModalView, TouchStack, ViewData, ViewFrame, ViewSubviews, ViewTest, ViewTouch, view,
    },
    ui_test::inject_touches,
};
use log::debug;

#[view]
struct TouchStackTestView {
    // #[text = a]
    #[init]
    button:  Button,
    // #[text = b]
    button2: Button,
}

impl ViewTest for TouchStackTestView {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        assert_eq!(TouchStack::dump(), vec![vec!["Layer: Root view".to_string()]]);

        let mut button = from_main(move || view.add_view::<Button>());

        assert_eq!(TouchStack::dump(), vec![vec!["Layer: Root view".to_string()]]);

        button.on_tap(|| {});

        assert_eq!(
            TouchStack::dump(),
            vec![vec!["Layer: Root view", button.view_label()]],
        );

        from_main(move || button.remove_from_superview());

        assert_eq!(TouchStack::dump(), vec![vec!["Layer: Root view".to_string()]]);

        view.button.on_tap(|| {});

        assert_eq!(
            TouchStack::dump(),
            vec![vec!["Layer: Root view", view.button.view_label()]],
        );

        view.button2.on_tap(|| {});

        assert_eq!(
            TouchStack::dump(),
            vec![vec![
                "Layer: Root view",
                view.button.view_label(),
                view.button2.view_label(),
            ]],
        );

        view.button.disable_touch();
        view.button2.disable_touch();

        assert_eq!(TouchStack::dump(), vec![vec!["Layer: Root view".to_string()]]);

        let alert = from_main(|| Alert::prepare_modally_with_input("Hello".to_string()));
        wait_for_next_frame();

        assert_eq!(
            TouchStack::dump(),
            vec![
                vec!["Layer: Root view".to_string()],
                vec!["Layer: Alert".to_string(), "Alert.ok_button: Button".to_string()],
            ],
        );

        // The alert sizes itself to its message, so the OK tap point
        // comes from its frame: the button is the alert's bottom row.
        let frame = from_main(move || *alert.frame());
        let (x, y) = (frame.center().x, frame.max_y() - 22.0);
        inject_touches(format!("{x:.0} {y:.0} b\n{x:.0} {y:.0} e"));

        assert_eq!(TouchStack::dump(), vec![vec!["Layer: Root view".to_string()]]);

        debug!("Touch stack test: OK");

        Ok(())
    }
}
