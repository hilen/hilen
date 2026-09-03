use anyhow::Result;
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{BLUE, Container, GREEN, Hover, Setup, ViewData, ViewSubviews, ViewTest, ViewTouch, view},
    ui_test::inject_touches,
};

/// The hovered view can die without a cursor event: a store update
/// rebuilds rows under a stationary cursor. The engine then re-picks
/// under the last cursor position, so the replacement row gets its
/// enter without waiting for a mouse move, like CSS hover.
#[view]
struct HoverRemoval {
    log: Vec<(&'static str, bool)>,

    #[init]
    host: Container,
}

impl Setup for HoverRemoval {
    fn setup(self: Weak<Self>) {
        self.host.place().tl(20).size(200, 100);
        self.build_row("first");
    }
}

impl HoverRemoval {
    fn build_row(self: Weak<Self>, name: &'static str) {
        let row = self.host.add_view::<Container>();
        row.set_color(BLUE);
        row.place().back();
        row.enable_hover();
        row.touch().hovered.val(self, move |hovered| {
            let mut this = self;
            this.log.push((name, hovered));
            row.set_color(if hovered { GREEN } else { BLUE });
        });
    }
}

impl ViewTest for HoverRemoval {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        // Cursor position and hover state can leak from previous tests.
        from_main(Hover::clear);
        from_main(move || {
            let mut this = view;
            this.log.clear();
        });

        inject_touches("120 70 m");

        from_main(move || {
            assert_eq!(view.log, vec![("first", true)]);
        });

        // A rebuild under the stationary cursor: the hovered row dies and
        // a replacement takes its place. The replacement hovers on its
        // own, no mouse move needed.
        from_main(move || {
            view.host.remove_all_subviews();
            view.build_row("second");
        });
        wait_for_next_frame();
        wait_for_next_frame();

        from_main(move || {
            assert_eq!(view.log, vec![("first", true), ("second", true)]);
        });

        // A rebuild to nothing settles: the dead pointer drops, no exit
        // fires for a view that no longer exists, and later frames stay
        // quiet instead of re-picking forever.
        from_main(move || view.host.remove_all_subviews());
        wait_for_next_frame();
        wait_for_next_frame();
        wait_for_next_frame();

        from_main(move || {
            assert_eq!(view.log, vec![("first", true), ("second", true)]);
        });

        // With no hover history the re-pick stays off: a row appearing
        // under the stationary cursor waits for the next mouse move.
        from_main(move || view.build_row("third"));
        wait_for_next_frame();
        wait_for_next_frame();

        from_main(move || {
            assert_eq!(view.log, vec![("first", true), ("second", true)]);
        });

        inject_touches("121 70 m");

        from_main(move || {
            assert_eq!(view.log, vec![("first", true), ("second", true), ("third", true)]);
        });

        Ok(())
    }
}
