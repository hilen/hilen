use anyhow::Result;
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{Alert, Button, TouchStack, ViewData, ViewSubviews, ViewTest, ViewTouch, view},
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

        Alert::show("Hello");

        {
            use hilen::ui::{UIManager, ViewFrame};
            let root = from_main(|| UIManager::root_view().frame().size);
            let scale = from_main(UIManager::scale);
            log::error!("PROBE root {root:?} scale {scale}");
        }

        assert_eq!(
            TouchStack::dump(),
            vec![
                vec!["Layer: Root view".to_string()],
                vec!["Layer: Alert".to_string(), "Alert.ok_button: Button".to_string()],
            ],
        );

        inject_touches(
            r"
            320  383  b
            320  383  e
    ",
        );

        assert_eq!(TouchStack::dump(), vec![vec!["Layer: Root view".to_string()]]);

        debug!("Touch stack test: OK");

        Ok(())
    }
}
