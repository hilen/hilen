use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    level::{LevelSetup, LevelTest, LevelTestView, level},
    refs::Weak,
    ui::{Button, Focus, NamedKey, View, ViewData, ViewSubviews, WeakView},
    ui_test::{checkpoint, inject_named_key},
};

/// A level that only draws behind the views leaves the arrow keys to the
/// key focus. The focus was once off while any level ran, so a screen with
/// a level as its backdrop could not be driven with a remote. A level that
/// takes the keys, the default, still keeps the ring away.
#[level]
#[derive(Default)]
struct LevelLeavesKeys {
    game:   bool,
    button: WeakView,
}

impl LevelSetup for LevelLeavesKeys {
    fn takes_keys(&self) -> bool {
        self.game
    }
}

fn ring_on_button(level: Weak<LevelLeavesKeys>) -> bool {
    from_main(move || Focus::focused().raw() == level.button.raw())
}

impl LevelTest for LevelLeavesKeys {
    fn overlay(mut level: Weak<Self>, view: Weak<LevelTestView>) {
        let button = view.add_view::<Button>();
        button.set_text("over a level").set_text_size(24);
        button.set_color("#cfd8e3").place().center().size(300, 80);
        button.on_tap(|| {});
        level.button = button.weak_view();
    }

    fn perform_test(mut level: Weak<Self>) -> Result<()> {
        wait_for_next_frame();

        inject_named_key(NamedKey::ArrowDown);
        wait_for_next_frame();
        ensure!(
            ring_on_button(level),
            "the ring did not show over a level that leaves the keys"
        );
        checkpoint("the ring is on the button over the level")?;

        from_main(move || {
            level.game = true;
            Focus::hide();
        });
        inject_named_key(NamedKey::ArrowDown);
        wait_for_next_frame();
        ensure!(
            !ring_on_button(level),
            "the ring showed over a level that takes the keys"
        );

        Ok(())
    }
}
