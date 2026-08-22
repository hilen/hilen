use std::time::Duration;

use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::{Own, Weak},
    time::Instant,
    ui::{
        BLACK, Button, CLEAR, ImageView, Label, NavigationView, Setup, View, ViewController, ViewData,
        ViewSubviews, ViewTest, WHITE, view,
    },
    ui_test::{check_colors, inject_touches},
};

#[view]
struct NavigationRich {
    #[init]
    title:    Label,
    subtitle: Label,
    folder:   ImageView,
    to_next:  Button,
    menu:     Button,
}

impl Setup for NavigationRich {
    fn setup(self: Weak<Self>) {
        self.set_color("#27AE60");

        self.title.set_text("Home");
        self.title.set_text_size(40);
        self.title.set_text_color(WHITE);
        self.title.set_color(CLEAR);
        self.title.place().t(24).center_x().size(360, 56);

        self.subtitle.set_text("Navigation stack test");
        self.subtitle.set_text_size(20);
        self.subtitle.set_text_color(BLACK);
        self.subtitle.set_color(CLEAR);
        self.subtitle.place().below(self.title, 12);

        self.folder.set_image("folder.png");
        self.folder.place().size(160, 160).center();

        self.to_next.set_text("Push");
        self.to_next.set_color("#F1C40F");
        self.to_next.place().center_x().b(80).size(220, 64);
        self.to_next.on_tap(move || {
            self.navigation().push(PushedRich::new());
        });

        self.menu.set_text("Menu");
        self.menu.place().l(24).b(24).size(120, 44);
    }
}

#[view]
struct PushedRich {
    #[init]
    title:    Label,
    subtitle: Label,
    arrow:    ImageView,
    back:     Button,
    footer:   Label,
}

impl Setup for PushedRich {
    fn setup(self: Weak<Self>) {
        self.set_color("#2C3E50");

        self.title.set_text("Pushed");
        self.title.set_text_size(40);
        self.title.set_text_color(WHITE);
        self.title.set_color(CLEAR);
        self.title.place().t(24).center_x().size(360, 56);

        self.subtitle.set_text("Tap back to pop");
        self.subtitle.set_text_size(20);
        self.subtitle.set_text_color(WHITE);
        self.subtitle.set_color(CLEAR);
        self.subtitle.place().below(self.title, 12);

        self.arrow.set_image("arrow.png");
        self.arrow.place().size(160, 160).center();

        self.back.set_text("Back");
        self.back.set_color("#E74C3C");
        self.back.place().center_x().b(80).size(220, 64);
        self.back.on_tap(move || {
            self.navigation().pop();
        });

        self.footer.set_text("stack depth two");
        self.footer.set_text_size(18);
        self.footer.set_text_color(WHITE);
        self.footer.set_color(CLEAR);
        self.footer.place().l(24).b(24).size(160, 40);
    }
}

impl ViewTest for NavigationRich {
    fn make_root(view: Own<Self>) -> Own<dyn View> {
        NavigationView::with_view(view)
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        check_colors(HOME)?;

        inject_touches(
            "
            300 488 b
            300 488 e
        ",
        );

        // Push hides the previous view when its animation finishes.
        wait_until("push", || from_main(move || view.is_hidden()))?;

        check_colors(PUSHED)?;

        inject_touches(
            "
            300 488 b
            300 488 e
        ",
        );

        // Pop removes the popped view when its animation finishes.
        wait_until("pop", move || {
            from_main(move || !view.is_hidden() && view.navigation().subviews().len() == 1)
        })?;

        // The home view must come back exactly as it was.
        check_colors(HOME)?;

        Ok(())
    }
}

/// The bound counts a suspended gap as one frame, a browser stops rendering
/// while the window is covered and a paused run must resume, not fail.
fn wait_until(action: &str, mut done: impl FnMut() -> bool) -> Result<()> {
    let mut waited = Duration::ZERO;
    while !done() {
        ensure!(
            waited < Duration::from_secs(10),
            "{action} animation never finished"
        );
        let frame = Instant::now();
        wait_for_next_frame();
        waited += frame.elapsed().min(Duration::from_millis(100));
    }
    Ok(())
}

const HOME: &str = "";

const PUSHED: &str = "";
