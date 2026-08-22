use anyhow::Result;

use crate::{
    deps::{hreads::from_main, refs::Weak},
    gm::color::{BLUE, RED},
    ui::{Container, Setup, ViewData, ViewTest, ViewTouch, view},
    ui_test::inject_touches,
};

/// A view hidden during an active press must lose the touch capture.
/// Before the fix it kept it forever: hover moves triggered its events
/// after unhiding and it consumed other views' touch ends.
#[view]
struct HiddenTouch {
    a_moved: u32,
    a_up:    u32,
    b_up:    u32,

    #[init]
    b: Container,
    a: Container,
}

impl Setup for HiddenTouch {
    fn setup(mut self: Weak<Self>) {
        self.b.set_color(BLUE).place().size(200, 200).tl(300);
        self.b.enable_touch();
        self.b.touch().up_inside.sub(self, move || self.b_up += 1);

        // `a` is enabled after `b`, so touch dispatch checks it first.
        self.a.set_color(RED).place().size(200, 200).tl(0);
        self.a.enable_touch();
        self.a.touch().moved.val(move |_| self.a_moved += 1);
        self.a.touch().up_inside.sub(self, move || self.a_up += 1);
    }
}

impl ViewTest for HiddenTouch {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        inject_touches(
            "
            400 400 b
            400 400 e
        ",
        );
        assert_eq!(from_main(move || view.b_up), 1);

        // Press `a`, hide it mid-press, release elsewhere.
        inject_touches("100 100 b");
        from_main(move || {
            view.a.set_hidden(true);
        });
        inject_touches("100 100 e");

        from_main(move || {
            view.a.set_hidden(false);
        });

        // A hover move must not trigger the old captor.
        inject_touches("100 100 m");
        assert_eq!(from_main(move || view.a_moved), 0);

        // A tap on `b` must reach `b`, not the old captor.
        inject_touches(
            "
            400 400 b
            400 400 e
        ",
        );
        assert_eq!(from_main(move || (view.a_up, view.b_up)), (0, 2));

        Ok(())
    }
}
