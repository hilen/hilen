use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{
        Button, Focus, Label, NamedKey, Setup, Tooltip, View, ViewData, ViewFrame, ViewTest, ViewTooltip,
        view,
    },
    ui_test::{checkpoint, inject_named_key, wait_for_tooltip},
};

/// The focus ring that rests on a view at the bottom of the screen shows its
/// tooltip 8 points above the view, since below it would leave the screen
/// and sliding it up would cover the view.
#[view]
struct KeyFocusTooltipAbove {
    #[init]
    title:  Label,
    state:  Label,
    bottom: Button,
}

impl Setup for KeyFocusTooltipAbove {
    fn setup(self: Weak<Self>) {
        self.title.set_text("no room below, the tooltip goes above").set_text_size(20);
        self.title.place().lrt(10).h(40);

        self.state.set_text("no tooltip").set_text_size(20);
        self.state.place().lr(10).t(60).h(40);

        self.bottom.set_text("at the bottom, has a tooltip").set_text_size(18);
        self.bottom.set_color("#cfd8e3").set_corner_radius(10);
        self.bottom.set_frame((40, 520, 280, 60));
        self.bottom.set_tooltip("2026-11-18 13:40 UTC");
        self.bottom.on_tap(|| {});
    }
}

impl ViewTest for KeyFocusTooltipAbove {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        inject_named_key(NamedKey::ArrowRight);
        wait_for_next_frame();
        ensure!(
            from_main(move || Focus::focused().raw() == view.bottom.weak_view().raw()),
            "the first key puts the ring on the button at the bottom"
        );

        wait_for_tooltip();
        let (shown, frame) = from_main(move || {
            let shown = Tooltip::shown();
            view.state.set_text(if shown.is_ok() {
                "tooltip shown above"
            } else {
                "no tooltip"
            });
            (*shown.frame(), *view.bottom.absolute_frame())
        });
        ensure!(
            shown.origin == (frame.x(), frame.y() - 8.0 - shown.height()).into(),
            "8 points above the button, got {shown:?} for the button at {frame:?}"
        );
        checkpoint("the ring rests on the bottom button, its date shows above it")?;

        Ok(())
    }
}
