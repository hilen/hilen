use anyhow::{Result, ensure};

use super::partial_frames::{partial_frames_possible, same_pixels};
use crate::{
    self as hilen, AppRunner,
    deps::{hreads::from_main, refs::Weak},
    gm::{
        LossyConvert,
        color::{BLUE, GREEN, LIGHT_GRAY, ORANGE, RED},
    },
    ui::{Container, Setup, UIManager, ViewData, ViewFrame, ViewSubviews, ViewTest, damage, view},
};

const BARS: usize = 6;
const BAR_WIDTH: f32 = 560.0;
const BAR_HEIGHT: f32 = 60.0;

/// A view removed early in the tree moves the depth of every view after
/// it. Those show the same pixels, so a removal repaints only where the
/// view was. The depth was in the paint key, and a removed mark of
/// `Partial frames` repainted 95432 pixels.
#[view]
struct RemovalRepaintsLittle {
    bars: Vec<Weak<Container>>,

    #[init]
    mark: Container,
}

impl Setup for RemovalRepaintsLittle {
    fn setup(mut self: Weak<Self>) {
        self.set_color(LIGHT_GRAY);
        self.mark.set_color(RED).set_corner_radius(4);
        self.mark.set_frame((20, 20, 40, 40));

        // Added after the mark, so each of them is in front of it.
        for (index, color) in [BLUE, GREEN, ORANGE, BLUE, GREEN, ORANGE].into_iter().enumerate() {
            let bar = self.add_view::<Container>();
            let top = 100.0 + 70.0 * f32::from(u8::try_from(index).unwrap_or_default());
            bar.set_color(color).set_frame((20.0, top, BAR_WIDTH, BAR_HEIGHT));
            self.bars.push(bar);
        }
    }
}

impl ViewTest for RemovalRepaintsLittle {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        ensure!(from_main(move || view.bars.len()) == BARS);
        // The first frames of a fixture are whole, its views were never
        // drawn.
        AppRunner::take_screenshot()?;
        AppRunner::take_screenshot()?;

        let before = from_main(move || {
            let mut mark = view.mark;
            mark.remove_from_superview();
            damage::frames()
        });
        let partial = AppRunner::take_screenshot()?;
        let after = from_main(damage::frames);

        if partial_frames_possible(after) {
            ensure!(
                after.whole == before.whole,
                "a removal drew a whole frame: {}",
                after.why
            );
            let scale = from_main(UIManager::scale);
            let bar: u32 = (BAR_WIDTH * BAR_HEIGHT * scale * scale).lossy_convert();
            let drawn = after.pixels - before.pixels;
            ensure!(
                drawn < u64::from(bar),
                "removing a mark of 40 by 40 drew {drawn} pixels, 1 bar after it is {bar}"
            );
        }

        from_main(|| damage::whole_next("the test compares with a whole frame"));
        let whole = AppRunner::take_screenshot()?;
        same_pixels(&partial, &whole, "the mark is gone")
    }
}
