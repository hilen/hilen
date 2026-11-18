use std::time::{Duration, Instant};

use anyhow::{Result, bail, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::color::Color,
    refs::{Own, Weak},
    ui::{
        Button, Label, NamedKey, Popover, Rect, Setup, TextAlignment, View, ViewData, ViewFrame, ViewTest,
        view,
    },
    ui_test::{check_colors, inject_named_key, inject_touches},
};

const INK: Color = Color::hex("#20232A");

/// Where the sheet rests: as wide as the canvas, the card of 200 and the
/// grab bar room of 20 at the bottom.
fn resting() -> Rect {
    Rect::new(0.0, 480.0, 400.0, 220.0)
}

const SHEET: &str = r"
              40   16 - #212830
             184   16 - #262e38
              80   20 - #476377
             136   20 - #1a1c22
             152   20 - #476377
             188   20 - #476377
             220   20 - #476377
             116   24 - #1a1c22
             156   60 - #a6adb6
             268   60 - #a6adb6
             188   76 - #a6adb6
             196   76 - #a6adb6
             192   80 - #a6adb6
             212   80 - #a6adb6
             132   88 - #a6adb6
             248   88 - #a6adb6
             392  144 - #476377
               4  180 - #476377
             192  280 - #476377
             392  296 - #476377
               4  340 - #476377
             184  488 - #c7c7cc
             192  488 - #c7c7cc
             200  488 - #c7c7cc
             212  488 - #c7c7cc
             392  488 - #ffffff
               8  516 - #ffffff
             200  600 - #ffffff
             208  600 - #ffffff
             236  600 - #20232a
               4  692 - #ffffff
             392  692 - #ffffff
";

fn card() -> Own<dyn View> {
    let label = Label::new();
    label
        .set_text("sheet content")
        .set_text_size(16)
        .set_alignment(TextAlignment::Center)
        .set_text_color(INK);
    label.set_size(300, 200);
    label
}

/// On a narrow window a popover is a sheet: it slides up from the bottom
/// over a scrim, as wide as the window. A short drag down puts it back, a
/// long one closes it, and so do a tap on the scrim and Escape.
#[view]
struct PopoverSheet {
    #[init]
    status: Label,
    open:   Button,
}

impl Setup for PopoverSheet {
    fn setup(self: Weak<Self>) {
        self.status.set_text("nothing open").set_text_size(14).set_text_color(INK);
        self.status.place().t(8).lr(16).h(22);
        self.open.set_text("Open").set_text_size(14);
        self.open.set_color(Color::hex("#CFD8E3"));
        self.open.place().t(60).center_x().size(140, 32);
        self.open.on_tap(move || {
            Popover::show(card(), self.open);
        });
    }
}

impl PopoverSheet {
    fn say(view: Weak<Self>, text: &'static str) {
        from_main(move || {
            view.status.set_text(text);
        });
    }

    /// Waits until the slide ends with the sheet at `frame`.
    fn settled(frame: Rect) -> Result<()> {
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(3) {
            if from_main(|| *Popover::open().frame()) == frame {
                return Ok(());
            }
            wait_for_next_frame();
        }
        let got = from_main(|| *Popover::open().frame());
        bail!("the sheet stayed at {got:?}, not at {frame:?}")
    }

    fn closed() -> Result<()> {
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(3) {
            if from_main(|| Popover::open().is_null()) {
                return Ok(());
            }
            wait_for_next_frame();
        }
        bail!("the sheet is still open")
    }
}

impl ViewTest for PopoverSheet {
    fn canvas() -> (u32, u32) {
        (400, 700)
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        inject_touches("200 76 b\n200 76 e");
        ensure!(
            from_main(|| Popover::open().is_sheet()),
            "a narrow window gets a sheet"
        );
        Self::settled(resting())?;
        Self::say(view, "sheet up, the page dims behind it");
        check_colors(SHEET)?;

        inject_touches("200 490 b\n200 530 m\n200 530 e");
        Self::settled(resting())?;
        Self::say(view, "a short drag down put it back");

        inject_touches("200 490 b\n200 640 m\n200 640 e");
        Self::closed()?;
        Self::say(view, "a long drag down closed it");

        inject_touches("200 76 b\n200 76 e");
        Self::settled(resting())?;
        inject_touches("200 300 b\n200 300 e");
        ensure!(
            from_main(|| Popover::open().is_null()),
            "a tap on the scrim closes it"
        );

        inject_touches("200 76 b\n200 76 e");
        Self::settled(resting())?;
        inject_named_key(NamedKey::Escape);
        ensure!(from_main(|| Popover::open().is_null()), "Escape closes it");
        Self::say(view, "closed by the scrim and by Escape");
        Ok(())
    }
}
