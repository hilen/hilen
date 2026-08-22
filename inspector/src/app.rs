use hilen::{
    App,
    refs::Own,
    ui::{Setup, Size, View},
};

use crate::ui::MainScreen;

#[derive(Default)]
pub struct InspectorApp;

impl App for InspectorApp {
    fn make_root_view(&self) -> Own<dyn View> {
        MainScreen::new()
    }

    fn initial_size(&self) -> Size {
        (1200, 1200).into()
    }

    fn after_launch(&self) {
        // hilen::ui::UIManager::enable_debug_frames();
    }
}
