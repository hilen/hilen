use anyhow::Result;
use hilen::{
    dispatch::from_main,
    inspect::{ViewRepr, ViewToInspect, views::PlacerView},
    refs::{Own, Weak},
    ui::{Container, Setup, TURQUOISE, ViewData, ViewFrame, ViewTest, view},
};

#[view]
struct PlacerViewTest {
    repr: Own<ViewRepr>,

    #[init]
    placer_view: PlacerView,
    view:        Container,
}

impl Setup for PlacerViewTest {
    fn setup(self: Weak<Self>) {
        hilen::ui::UIManager::override_scale(2.0);

        self.placer_view.set_size(200, 800);

        self.view.set_color(TURQUOISE);
        self.view.place().center().size(80, 200);
    }
}

impl ViewTest for PlacerViewTest {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        // UIManager::enable_debug_frames();

        from_main(move || {
            let mut view = view;
            view.repr = view.view.view_to_inspect();
            view.placer_view.set_view(view.repr.weak());
        });

        // hilen::ui_test::record_ui_test();

        from_main(|| {
            hilen::ui::UIManager::override_scale(1.0);
        });

        Ok(())
    }
}
