use anyhow::Result;

use crate::{
    deps::{hreads::from_main, refs::Weak},
    gm::color::{BLUE, RED},
    ui::{Container, Setup, TouchStack, View, ViewData, ViewFrame, ViewSubviews, ViewTest, ViewTouch, view},
    ui_test::inject_touches,
};

/// Touch dispatch goes by registration order, not depth, so a view
/// registered after a full-screen overlay steals its taps. The public
/// `TouchStack::push_layer` makes the overlay the only touch target,
/// migrating registrations already under it into the layer, and
/// `pop_layer` hands survivors back so the overlay can reopen.
#[view]
struct OverlayTouchLayer {
    under_taps: u32,
    over_taps:  u32,
    late_taps:  u32,

    #[init]
    under:   Container,
    overlay: Container,
}

impl Setup for OverlayTouchLayer {
    fn setup(mut self: Weak<Self>) {
        self.under.set_color(BLUE);
        self.under.set_frame((0, 0, 200, 200));
        self.under.enable_touch();
        self.under.touch().up_inside.sub(self, move || self.under_taps += 1);

        // The overlay's child registers its touch before any layer
        // exists, push_layer must migrate it in.
        self.overlay.set_frame((0, 0, 200, 200));
        self.overlay.set_hidden(true);
        let child = self.overlay.add_view::<Container>();
        child.set_color(RED);
        child.set_frame((0, 0, 200, 200));
        child.enable_touch();
        child.touch().up_inside.sub(self, move || self.over_taps += 1);
    }
}

impl ViewTest for OverlayTouchLayer {
    fn perform_test(mut view: Weak<Self>) -> Result<()> {
        let tap = || inject_touches("100 100 b\n100 100 e");

        // Hidden overlay: the base view receives the tap.
        tap();
        assert_eq!(from_main(move || (view.under_taps, view.over_taps)), (1, 0));

        from_main(move || {
            view.overlay.set_hidden(false);
            TouchStack::push_layer(view.overlay.weak_view());

            // A view registered after the push lands in the base layer
            // and must not steal from the overlay. This is the settings
            // sheet vs the constantly rebuilding sidebar.
            let late = view.superview().add_view::<Container>();
            late.set_frame((0, 0, 200, 200));
            late.enable_touch();
            late.touch().up_inside.sub(view, move || view.late_taps += 1);
        });

        tap();
        assert_eq!(
            from_main(move || (view.under_taps, view.over_taps, view.late_taps)),
            (1, 1, 0),
            "the overlay layer must receive the tap"
        );

        // Closed overlay: the late view is the newest base registration
        // and wins the tap.
        from_main(move || {
            TouchStack::pop_layer(view.overlay.weak_view());
            view.overlay.set_hidden(true);
        });
        tap();
        assert_eq!(
            from_main(move || (view.under_taps, view.over_taps, view.late_taps)),
            (1, 1, 1),
            "after the pop the base layer works again"
        );

        // Reopened overlay: its registrations survived the pop through
        // the merge back and migrate up again.
        from_main(move || {
            view.overlay.set_hidden(false);
            TouchStack::push_layer(view.overlay.weak_view());
        });
        tap();
        assert_eq!(
            from_main(move || (view.under_taps, view.over_taps, view.late_taps)),
            (1, 2, 1),
            "a reopened overlay must keep receiving taps"
        );

        from_main(move || {
            TouchStack::pop_layer(view.overlay.weak_view());
            view.overlay.set_hidden(true);
        });

        Ok(())
    }
}
