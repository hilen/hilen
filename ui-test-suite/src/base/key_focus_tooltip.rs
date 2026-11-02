use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{
        Button, Focus, Label, NamedKey, Setup, Tooltip, View, ViewData, ViewFrame, ViewTest, ViewTooltip,
        view,
    },
    ui_test::{checkpoint, inject_named_key, inject_touches, wait_for_tooltip},
};

/// The focus ring that rests on a view shows its tooltip, under the view, the
/// way a resting cursor does. A ring that moves on or hides takes it away.
#[view]
struct KeyFocusTooltip {
    #[init]
    title: Label,
    date:  Button,
    plain: Button,
    state: Label,
}

fn press(key: NamedKey) {
    inject_named_key(key);
    wait_for_next_frame();
}

impl KeyFocusTooltip {
    fn shown(self: Weak<Self>) -> bool {
        from_main(move || {
            let shown = Tooltip::shown().is_ok();
            self.state.set_text(if shown { "tooltip shown" } else { "no tooltip" });
            shown
        })
    }
}

impl Setup for KeyFocusTooltip {
    fn setup(self: Weak<Self>) {
        self.title.set_text("the ring shows the tooltip of its view").set_text_size(20);
        self.title.place().lrt(10).h(40);

        self.date.set_text("3 hours ago, has a tooltip").set_text_size(18);
        self.date.set_color("#cfd8e3").set_corner_radius(10);
        self.date.set_frame((40, 120, 240, 60));
        self.date.set_tooltip("2026-11-02 14:05 UTC");
        self.date.on_tap(|| {});

        self.plain.set_text("no tooltip").set_text_size(18);
        self.plain.set_color("#f0d060").set_corner_radius(10);
        self.plain.set_frame((320, 120, 240, 60));
        self.plain.on_tap(|| {});

        self.state.set_text("no tooltip").set_text_size(20);
        self.state.place().lrb(10).h(40);
    }
}

impl ViewTest for KeyFocusTooltip {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        press(NamedKey::ArrowRight);
        ensure!(
            from_main(move || Focus::focused().raw() == view.date.weak_view().raw()),
            "the first key puts the ring on the view with the tooltip"
        );
        ensure!(!view.shown(), "the tooltip waits for the delay");

        wait_for_tooltip();
        ensure!(view.shown(), "the ring rested on the view, its tooltip shows");
        from_main(move || {
            let frame = *view.date.absolute_frame();
            let shown = Tooltip::shown();
            assert_eq!(shown.frame().origin, (frame.x(), frame.max_y() + 8.0).into());
        });
        checkpoint("the ring rests on the first button, its date shows under it")?;

        press(NamedKey::ArrowRight);
        ensure!(!view.shown(), "the ring moved on, the tooltip went with it");
        wait_for_tooltip();
        ensure!(!view.shown(), "a view with no tooltip shows none");
        checkpoint("the ring is on the second button, no tooltip")?;

        press(NamedKey::ArrowLeft);
        wait_for_tooltip();
        ensure!(view.shown(), "back on the view, the tooltip shows again");

        // A pointer press takes over from the keys and hides the ring.
        inject_touches("300 400 b\n300 400 e");
        wait_for_next_frame();
        ensure!(!view.shown(), "the ring hid, so did its tooltip");
        wait_for_tooltip();
        ensure!(!view.shown(), "no tooltip comes back after the ring hid");

        Ok(())
    }
}
