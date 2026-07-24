use std::time::{Duration, Instant};

use anyhow::{Result, ensure};
use parking_lot::Mutex;
use test_engine::{
    dispatch::wait_for_next_frame,
    refs::{Own, Weak},
    ui::{
        BLACK, BLUE, Button, CLEAR, CheckBox, CircleView, Container, GREEN, ImageView, Label, NavigationView,
        NumberView, ProgressView, Setup, Shadow, Slider, Switch, TURQUOISE, View, ViewController, ViewData,
        ViewTest, WHITE, view,
    },
    ui_test::{check_colors, inject_touches, set_record_probe_count},
};

// Present creates the next view inside a tap closure, so each setup publishes
// its weak pointer for the test to wait on.
static CURRENT_FIRST: Mutex<Option<Weak<PresentRich>>> = Mutex::new(None);
static CURRENT_PRESENTED: Mutex<Option<Weak<PresentedRich>>> = Mutex::new(None);

#[view]
struct PresentRich {
    #[init]
    title:    Label,
    subtitle: Label,
    palm:     ImageView,
    check:    CheckBox,
    toggle:   Switch,
    progress: ProgressView,
    number:   NumberView,
    card:     Container,
    circle:   CircleView,
    slider:   Slider,
    hint:     Label,
    to_next:  Button,
    later:    Button,
}

impl Setup for PresentRich {
    fn setup(mut self: Weak<Self>) {
        *CURRENT_FIRST.lock() = Some(self);

        self.set_color("#E67E22");

        self.title.set_text("Warm screen");
        self.title.set_text_size(40);
        self.title.set_text_color(WHITE);
        self.title.set_color(CLEAR);
        self.title.place().t(24).center_x().size(360, 56);

        self.subtitle.set_text("First view of present test");
        self.subtitle.set_text_size(20);
        self.subtitle.set_text_color(BLACK);
        self.subtitle.set_color(CLEAR);
        self.subtitle.place().below(self.title, 12);

        self.palm.set_image("palm.png");
        self.palm.place().size(150, 150).t(160).l(420);

        self.check.set_on(true);
        self.check.place().size(44, 44).t(160).l(24);

        self.toggle.set_on(true);
        self.toggle.place().size(80, 44).t(224).l(24);

        self.progress.set_progress(0.4);
        self.progress.place().size(150, 24).t(292).l(24);

        self.number.set_value(13);
        self.number.place().size(60, 84).t(340).l(24);

        self.card.set_gradient(GREEN, BLUE);
        self.card.set_corner_radius(16);
        self.card.place().size(120, 90).t(160).l(240);

        self.circle.set_radius(30);
        self.circle.set_color(TURQUOISE);
        self.circle.place().t(280).l(268);

        self.slider.set_range(0, 50).set_value(20);
        self.slider.place().size(44, 160).t(330).l(480);

        self.hint.set_text("slide up demo");
        self.hint.set_text_size(18);
        self.hint.set_text_color(WHITE);
        self.hint.set_color(CLEAR);
        self.hint.place().r(24).b(24).size(160, 40);

        self.to_next.set_text("Present");
        self.to_next.set_color("#F1C40F");
        self.to_next.set_shadow(Shadow::default());
        self.to_next.place().center_x().b(80).size(220, 64);
        self.to_next.on_tap(move || {
            self.present(PresentedRich::new());
        });

        self.later.set_text("Later");
        self.later.place().l(24).b(24).size(120, 44);
    }
}

#[view]
struct PresentedRich {
    #[init]
    title:    Label,
    body:     Label,
    ball:     ImageView,
    check:    CheckBox,
    toggle:   Switch,
    progress: ProgressView,
    number:   NumberView,
    card:     Container,
    circle:   CircleView,
    slider:   Slider,
    done:     Button,
    footer:   Label,
}

impl Setup for PresentedRich {
    fn setup(mut self: Weak<Self>) {
        *CURRENT_PRESENTED.lock() = Some(self);

        self.set_color("#8E44AD");

        self.title.set_text("Presented");
        self.title.set_text_size(40);
        self.title.set_text_color(WHITE);
        self.title.set_color(CLEAR);
        self.title.place().t(24).center_x().size(360, 56);

        self.body.set_text("It slid from the bottom");
        self.body.set_text_size(22);
        self.body.set_text_color(WHITE);
        self.body.set_color(CLEAR);
        self.body.place().below(self.title, 12);

        self.ball.set_image("ball.png");
        self.ball.place().br(24).size(140, 140);

        self.check.place().size(44, 44).t(160).l(24);

        self.toggle.set_on(true);
        self.toggle.place().size(80, 44).t(224).l(24);

        self.slider.set_range(0, 10).set_value(3);
        self.slider.place().size(44, 160).t(280).l(36);

        self.progress.set_progress(0.9);
        self.progress.place().size(150, 24).t(160).l(110);

        self.circle.set_radius(36);
        self.circle.set_color(GREEN);
        self.circle.place().t(210).l(140);

        self.number.set_value(99);
        self.number.place().size(60, 84).t(300).l(120);

        self.card.set_gradient(TURQUOISE, BLUE);
        self.card.set_corner_radius(20);
        self.card.place().size(130, 100).t(160).l(420);

        self.done.set_text("Done");
        self.done.set_color("#2ECC71");
        self.done.set_shadow(Shadow::default());
        self.done.place().center_x().b(80).size(220, 64);
        self.done.on_tap(move || {
            self.present(PresentRich::new());
        });

        self.footer.set_text("second view");
        self.footer.set_text_size(18);
        self.footer.set_text_color(WHITE);
        self.footer.set_color(CLEAR);
        self.footer.place().l(24).b(24).size(160, 40);
    }
}

impl ViewTest for PresentRich {
    /// Presenting only works from inside a navigation stack, so the root is the
    /// stack and the view under test is its first view.
    fn make_root(view: Own<Self>) -> Own<dyn View> {
        NavigationView::with_view(view)
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        set_record_probe_count(160);

        check_colors(FIRST)?;

        // Present and Done sit at the same spot, each lap slides there and back.
        let mut first = view;
        for _ in 0..4 {
            inject_touches(
                "
                300 488 b
                300 488 e
            ",
            );

            wait_until_gone(first)?;

            check_colors(PRESENTED)?;

            let presented = CURRENT_PRESENTED.lock().take().expect("present created no view");

            inject_touches(
                "
                300 488 b
                300 488 e
            ",
            );

            wait_until_gone(presented)?;

            check_colors(FIRST)?;

            first = CURRENT_FIRST.lock().take().expect("done created no first view");
        }

        ensure!(first.is_ok(), "a fresh first view must be alive at the end");

        Ok(())
    }
}

/// Present deallocates the old view when its animation finishes, so a dead
/// weak pointer is the completion signal.
fn wait_until_gone<T>(view: Weak<T>) -> Result<()> {
    let start = Instant::now();
    while view.is_ok() {
        ensure!(
            start.elapsed() < Duration::from_secs(10),
            "present animation never finished"
        );
        wait_for_next_frame();
    }
    Ok(())
}

const FIRST: &str = "";

const PRESENTED: &str = "";
