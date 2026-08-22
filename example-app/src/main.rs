#![allow(incomplete_features)]
#![feature(specialization)]
#![feature(arbitrary_self_types)]

use hilen::{
    App,
    refs::{Own, Weak},
    ui::{Label, Setup, U8Color, UIManager, View, ViewData, view},
};

#[view]
struct MainScreen {
    #[init]
    hello_label: Label,
}

impl Setup for MainScreen {
    fn setup(self: Weak<Self>) {
        UIManager::set_clear_color("#4E4D5C");

        self.hello_label
            .set_text("Hello Test Engine!")
            .set_color(U8Color::rgba(156, 149, 220, 255))
            .set_corner_radius(10)
            .set_border_color("#228CDB")
            .set_border_width(5)
            .set_text_size(40);

        self.hello_label.place().center().size(400, 80);
    }
}

#[derive(Default)]
struct ExampleApp;

impl App for ExampleApp {
    fn make_root_view(&self) -> Own<dyn View> {
        MainScreen::new()
    }
}

fn main() {
    ExampleApp::start();
}
