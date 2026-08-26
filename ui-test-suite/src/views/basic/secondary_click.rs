use std::{thread::sleep, time::Duration};

use anyhow::Result;
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{BLUE, Container, GREEN, Point, Setup, ViewData, ViewTest, ViewTouch, view},
    ui_test::{inject_long_press, inject_right_click, inject_touches},
};

#[view]
struct SecondaryClick {
    secondary: Vec<Point>,
    taps:      usize,

    #[init]
    target: Container,
    other:  Container,
}

impl Setup for SecondaryClick {
    fn setup(self: Weak<Self>) {
        self.target.set_color(BLUE);
        self.target.place().tl(100).size(200, 200);
        self.target.enable_touch();
        self.target.touch().secondary.val(self, move |touch| {
            let mut this = self;
            this.secondary.push(touch.position);
        });
        self.target.touch().up_inside.sub(self, move || {
            let mut this = self;
            this.taps += 1;
        });

        self.other.set_color(GREEN);
        self.other.place().t(400).l(100).size(200, 100);
        self.other.enable_touch();
    }
}

impl ViewTest for SecondaryClick {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        // A right click fires with the position in the view's own space.
        inject_right_click(150, 160);
        from_main(move || {
            assert_eq!(view.secondary, vec![Point::new(50.0, 60.0)]);
            assert_eq!(view.taps, 0);
        });

        // A right click on a view without a subscriber and on empty
        // space reaches nothing.
        inject_right_click(150, 450);
        inject_right_click(500, 500);
        from_main(move || assert_eq!(view.secondary.len(), 1));

        // A held touch fires the same event, and its release is no tap.
        inject_long_press(200, 200);
        from_main(move || {
            assert_eq!(
                view.secondary,
                vec![Point::new(50.0, 60.0), Point::new(100.0, 100.0)]
            );
            assert_eq!(view.taps, 0);
        });

        // A plain tap stays a tap.
        inject_touches("200 200 b\n200 200 e");
        from_main(move || {
            assert_eq!(view.secondary.len(), 2);
            assert_eq!(view.taps, 1);
        });

        // A touch that moves away before the threshold is a drag, not a hold.
        inject_touches("200 200 b\n230 230 m");
        sleep(Duration::from_secs_f32(0.8));
        wait_for_next_frame();
        inject_touches("230 230 e");
        from_main(move || {
            assert_eq!(view.secondary.len(), 2);
            assert_eq!(view.taps, 2);
        });

        Ok(())
    }
}
