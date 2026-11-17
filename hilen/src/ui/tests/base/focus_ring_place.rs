use anyhow::{Result, bail};

use crate::{
    self as hilen,
    deps::{
        hreads::{from_main, wait_for_next_frame},
        refs::Weak,
    },
    gm::color::{BLUE, WHITE},
    ui::{Button, Focus, Setup, UIManager, View, ViewData, ViewFrame, ViewTest, view},
    ui_test::checkpoint,
};

/// The focus ring sits around its view where the app views do not start at
/// the corner of the screen. On a phone they start below the status bar,
/// and the ring was once drawn lower than its view by the height of that
/// bar. The test drops the canvas, only the real screen has a safe area.
/// It also checks the safe area an app can ask for.
#[view]
struct FocusRingPlace {
    #[init]
    button: Button,
}

impl Setup for FocusRingPlace {
    fn setup(self: Weak<Self>) {
        self.button.set_text("under the ring").set_text_size(28).set_text_color(WHITE);
        self.button.set_color(BLUE).place().center().size(360, 80);
    }
}

impl ViewTest for FocusRingPlace {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        from_main(move || {
            UIManager::root_view().clear_test_canvas();
        });
        wait_for_next_frame();
        wait_for_next_frame();
        from_main(move || Focus::set(view.button.weak_view()));
        wait_for_next_frame();
        wait_for_next_frame();

        from_main(move || around(view))?;
        from_main(safe_area_inside)?;
        checkpoint("the ring is around the button")?;

        from_main(Focus::hide);
        Ok(())
    }
}

fn around(view: Weak<FocusRingPlace>) -> Result<()> {
    let button = *view.button.absolute_frame();
    let Some((ring, grow)) = Focus::ring_frame() else {
        bail!("no ring shows after Focus::set");
    };

    let off = (ring.x() - (button.x() - grow)).abs()
        + (ring.y() - (button.y() - grow)).abs()
        + (ring.width() - (button.width() + grow * 2.0)).abs()
        + (ring.height() - (button.height() + grow * 2.0)).abs();
    if off > 0.5 {
        bail!("the ring at {ring:?} is not around the button at {button:?}");
    }
    Ok(())
}

/// The safe area is a part of the app views. On an Apple TV the app views
/// are the whole screen and the safe area is smaller on every side.
fn safe_area_inside() -> Result<()> {
    let screen = UIManager::root_view().size();
    let safe = UIManager::safe_area();

    if safe.x() < 0.0
        || safe.y() < 0.0
        || safe.max_x() > screen.width + 0.5
        || safe.max_y() > screen.height + 0.5
    {
        bail!("the safe area {safe:?} is not inside the screen {screen:?}");
    }
    if cfg!(tvos)
        && (safe.x() <= 0.0
            || safe.y() <= 0.0
            || safe.max_x() >= screen.width
            || safe.max_y() >= screen.height)
    {
        bail!("the safe area {safe:?} of a TV reaches an edge of the screen {screen:?}");
    }
    Ok(())
}
