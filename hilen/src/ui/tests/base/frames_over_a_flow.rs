use anyhow::{Result, bail, ensure};

use super::partial_frames::{partial_frames_possible, same_pixels};
use crate::{
    self as hilen, AppRunner,
    deps::{
        hreads::{from_main, wait_for_next_frame},
        refs::Weak,
    },
    gm::color::{BLUE, GREEN, LIGHT_GRAY, RED, WHITE},
    ui::{
        Container, Label, ScrollView, Setup, TextAlignment, UIManager, ViewData, ViewFrame, ViewSubviews,
        ViewTest, damage, view,
    },
    ui_test::inject_touches,
};

const ROWS: u8 = 40;

// The way a person uses an app: a list, a screen opened from it, back, a
// fling, and after each a few frames with a small change, like a button
// that lights up. Each of those is compared with a whole frame of the
// same state. A frame with no change at all did not show the bug, a small
// change did. On an iPhone with 4
// samples a frame like that showed black or an older screen, while the
// simulator showed it right, see `docs/damage.md`. This test is in the
// phone lane of a release for that reason.
#[view]
struct FramesOverAFlow {
    title: Weak<Label>,

    #[init]
    list:   ScrollView,
    detail: Container,
    /// A small mark that changes in every compared frame, like a button
    /// that lights up under a finger.
    mark:   Container,
}

impl Setup for FramesOverAFlow {
    fn setup(mut self: Weak<Self>) {
        self.set_color(LIGHT_GRAY);

        self.list.place().back();
        self.list.set_content_size((600, f32::from(ROWS) * 50.0));
        for row in 0..ROWS {
            let label = self.list.add_view::<Label>();
            label.set_text(format!("row {}", row + 1)).set_text_size(20);
            label.set_alignment(TextAlignment::Left);
            label.set_color(if row % 2 == 0 { WHITE } else { LIGHT_GRAY });
            label.set_frame((0.0, f32::from(row) * 50.0, 600.0, 50.0));
        }

        self.detail.place().back();
        self.detail.set_color(WHITE);
        self.detail.set_hidden(true);
        self.title = self.detail.add_view::<Label>();
        self.title.set_text("an opened screen").set_text_size(28);
        self.title.set_color(BLUE.with_alpha(0.2));
        self.title.set_frame((20, 40, 400, 60));

        self.mark.set_color(RED).set_corner_radius(4);
        self.mark.set_frame((540, 20, 30, 30));
    }
}

/// Frames that each change only the small mark, each compared with a
/// whole frame. A partial frame draws the mark again and keeps the rest
/// from the frame before, the part a phone lost.
fn small_changes(view: Weak<FramesOverAFlow>, name: &str) -> Result<()> {
    for frame in 1..=3 {
        from_main(move || {
            let color = if frame % 2 == 0 { RED } else { GREEN };
            view.mark.set_color(color);
        });
        let partial = AppRunner::take_screenshot()?;
        from_main(|| damage::whole_next("the test compares with a whole frame"));
        let whole = AppRunner::take_screenshot()?;
        same_pixels(&partial, &whole, &format!("{name}, small change {frame}"))?;
    }
    Ok(())
}

fn fling_ends(view: Weak<FramesOverAFlow>) -> Result<()> {
    for _ in 0..2000 {
        if !from_main(move || view.list.is_scrolling()) {
            return Ok(());
        }
        wait_for_next_frame();
    }
    bail!("the fling did not stop in 2000 frames")
}

impl ViewTest for FramesOverAFlow {
    // A fling comes from a finger drag, which scrolls only with drag
    // scrolling on, the touch platform default.
    fn before_start() {
        UIManager::set_drag_scrolling(true);
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        // The first frames of a fixture are whole, its views were never
        // drawn.
        AppRunner::take_screenshot()?;
        AppRunner::take_screenshot()?;
        let start = from_main(damage::frames);

        small_changes(view, "the list")?;

        from_main(move || {
            view.list.set_hidden(true);
            view.detail.set_hidden(false);
        });
        AppRunner::take_screenshot()?;
        small_changes(view, "the opened screen")?;

        from_main(move || {
            view.detail.set_hidden(true);
            view.list.set_hidden(false);
        });
        AppRunner::take_screenshot()?;
        small_changes(view, "back on the list")?;

        inject_touches(
            "
            300 500 b
            300 460 m
            300 420 m
            300 420 e
        ",
        );
        fling_ends(view)?;
        small_changes(view, "the list after a fling")?;

        let end = from_main(damage::frames);
        if partial_frames_possible(end) {
            ensure!(
                end.partial > start.partial,
                "no partial frame was drawn, the test checked nothing"
            );
        }

        Ok(())
    }
}
