use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    scene::{SceneSetup, SceneTest, SceneTestView, scene},
    ui::{Button, Focus, NamedKey, View, ViewData, ViewSubviews, WeakView},
    ui_test::{checkpoint, inject_named_key},
};

/// A scene that only draws behind the views leaves the arrow keys to the
/// key focus. The focus was once off while any scene ran. A scene that
/// takes the keys, the default, still keeps the ring away.
#[scene]
#[derive(Default)]
struct SceneLeavesKeys {
    game:   bool,
    button: WeakView,
}

impl SceneSetup for SceneLeavesKeys {
    fn takes_keys(&self) -> bool {
        self.game
    }
}

fn ring_on_button(scene: Weak<SceneLeavesKeys>) -> bool {
    from_main(move || Focus::focused().raw() == scene.button.raw())
}

impl SceneTest for SceneLeavesKeys {
    fn overlay(mut scene: Weak<Self>, view: Weak<SceneTestView>) {
        let button = view.add_view::<Button>();
        button.set_text("over a scene").set_text_size(24);
        button.set_color("#cfd8e3").place().center().size(300, 80);
        button.on_tap(|| {});
        scene.button = button.weak_view();
    }

    fn perform_test(mut scene: Weak<Self>) -> Result<()> {
        wait_for_next_frame();

        inject_named_key(NamedKey::ArrowDown);
        wait_for_next_frame();
        ensure!(
            ring_on_button(scene),
            "the ring did not show over a scene that leaves the keys"
        );
        checkpoint("the ring is on the button over the scene")?;

        from_main(move || {
            scene.game = true;
            Focus::hide();
        });
        inject_named_key(NamedKey::ArrowDown);
        wait_for_next_frame();
        ensure!(
            !ring_on_button(scene),
            "the ring showed over a scene that takes the keys"
        );

        Ok(())
    }
}
