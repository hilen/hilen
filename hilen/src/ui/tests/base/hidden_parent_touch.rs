use anyhow::Result;

use crate::{
    deps::{hreads::from_main, refs::Weak},
    gm::color::{BLUE, RED},
    ui::{Container, Setup, ViewData, ViewSubviews, ViewTest, ViewTouch, view},
    ui_test::inject_touches,
};

#[view]
struct HiddenParentTouch {
    taps:  u32,
    child: Weak<Container>,

    #[init]
    parent: Container,
}

impl Setup for HiddenParentTouch {
    fn setup(mut self: Weak<Self>) {
        self.parent.set_color(BLUE).place().size(300, 300).tl(0);

        self.child = self.parent.add_view();
        self.child.set_color(RED).place().size(100, 100).tl(50);
        self.child.enable_touch();
        self.child.touch().up_inside.sub(self, move || self.taps += 1);
    }
}

impl ViewTest for HiddenParentTouch {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        let tap_child = || {
            inject_touches(
                "
                100 100 b
                100 100 e
            ",
            );
        };

        tap_child();
        assert_eq!(from_main(move || view.taps), 1);

        from_main(move || {
            view.parent.set_hidden(true);
        });

        tap_child();
        assert_eq!(from_main(move || view.taps), 1);

        from_main(move || {
            view.parent.set_hidden(false);
        });

        tap_child();
        assert_eq!(from_main(move || view.taps), 2);

        Ok(())
    }
}
