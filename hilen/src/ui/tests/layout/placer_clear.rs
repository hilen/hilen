use anyhow::{Result, ensure};

use crate::{
    self as hilen,
    deps::{
        hreads::{from_main, wait_for_next_frame},
        refs::Weak,
    },
    gm::color::{BLUE, WHITE},
    ui::{Container, Label, Setup, ViewData, ViewFrame, ViewTest, ui_test::checkpoint, view},
};

/// `place().clear()` drops a `custom` rule too. A screen that lays out
/// again for another width once kept its old custom rule after the clear,
/// and the view stayed at its old place.
#[view]
pub(crate) struct PlacerClearTest {
    #[init]
    target: Container,
    status: Label,
}

impl Setup for PlacerClearTest {
    fn setup(self: Weak<Self>) {
        self.target.set_color(BLUE);
        self.target.place().custom(|frame| {
            frame.origin.x = 300.0;
            frame.origin.y = 200.0;
            frame.size.width = 120.0;
            frame.size.height = 120.0;
        });
        self.status.set_text("box: custom rule at 300, 200").set_color(WHITE);
        self.status.place().bl(20).size(400, 40);
    }
}

impl ViewTest for PlacerClearTest {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        wait_for_next_frame();
        let before = from_main(move || view.target.frame().origin);
        ensure!(
            (before.x, before.y) == (300.0, 200.0),
            "the custom rule places the box, it is at {before:?}"
        );
        checkpoint("the box at 300, 200 from its custom rule")?;

        from_main(move || {
            view.target.place().clear().tl(20).size(120, 120);
            view.status.set_text("box: cleared, then top left 20");
        });
        wait_for_next_frame();
        wait_for_next_frame();
        let after = from_main(move || view.target.frame().origin);
        ensure!(
            (after.x, after.y) == (20.0, 20.0),
            "a cleared box takes only its new rules, it is at {after:?}"
        );
        checkpoint("the box at the top left, the custom rule is gone")?;
        Ok(())
    }
}
