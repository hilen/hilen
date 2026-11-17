use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{Button, Focus, Label, NamedKey, Setup, View, ViewData, ViewFrame, ViewTest, WeakView, view},
    ui_test::{checkpoint, inject_named_key, inject_touches},
};

/// A pointer that only moved hides the ring and the next arrow still moves
/// it, a remote with a pointer moves it with a small move of the hand. A
/// pointer that pressed took over, there the next arrow only brings the
/// ring back.
#[view]
struct KeyFocusPointer {
    #[init]
    title: Label,
    b1:    Button,
    b2:    Button,
    b3:    Button,
    state: Label,
}

impl KeyFocusPointer {
    fn focused(self: Weak<Self>) -> String {
        from_main(move || {
            let view: WeakView = Focus::focused();
            let name = if view.is_null() {
                "none".to_string()
            } else {
                [self.b1, self.b2, self.b3]
                    .iter()
                    .position(|button| button.weak_view().raw() == view.raw())
                    .map_or_else(|| "unknown".into(), |index| (index + 1).to_string())
            };
            self.state.set_text(format!("ring: {name}"));
            name
        })
    }

    fn press(self: Weak<Self>, key: NamedKey, expected: &str) -> Result<()> {
        inject_named_key(key);
        wait_for_next_frame();
        let focused = self.focused();
        ensure!(
            focused == expected,
            "after {key:?} the ring is on {focused}, expected {expected}"
        );
        wait_for_next_frame();
        Ok(())
    }

    fn pointer(self: Weak<Self>, touches: &'static str) -> Result<()> {
        inject_touches(touches);
        wait_for_next_frame();
        let focused = self.focused();
        ensure!(focused == "none", "the ring is on {focused} after the pointer");
        wait_for_next_frame();
        Ok(())
    }
}

impl Setup for KeyFocusPointer {
    fn setup(self: Weak<Self>) {
        self.title.set_text("a moved pointer costs no key").set_text_size(20);
        self.title.place().lrt(10).h(40);

        for (index, button) in [self.b1, self.b2, self.b3].into_iter().enumerate() {
            button.set_text(index + 1).set_text_size(24);
            button.set_color("#cfd8e3").set_corner_radius(10);
            button.set_frame((40 + index * 180, 200, 140_usize, 70_usize));
            button.on_tap(move || {
                self.state.set_text(format!("tapped {}", index + 1));
            });
        }

        self.state.set_text_size(20);
        self.state.place().lrb(10).h(40);
    }
}

impl ViewTest for KeyFocusPointer {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        view.press(NamedKey::ArrowRight, "1")?;
        checkpoint("the first key shows the ring on 1")?;
        view.press(NamedKey::ArrowRight, "2")?;

        view.pointer("300 30 m")?;
        checkpoint("a moved pointer hid the ring")?;
        view.press(NamedKey::ArrowRight, "3")?;
        checkpoint("the arrow after the move went on from 2 to 3")?;

        view.pointer("300 30 b\n300 30 e")?;
        checkpoint("a pressed pointer hid the ring")?;
        view.press(NamedKey::ArrowLeft, "3")?;
        checkpoint("the arrow after the press only brought the ring back on 3")?;

        view.pointer("310 30 m")?;
        view.pointer("310 30 b\n310 30 e")?;
        view.press(NamedKey::ArrowLeft, "3")?;
        checkpoint("a press after a move still only brings the ring back")?;

        view.pointer("300 30 b\n300 30 e")?;
        inject_touches("320 30 m");
        wait_for_next_frame();
        view.press(NamedKey::ArrowLeft, "3")?;
        checkpoint("a move with no ring to hide changes nothing")?;

        Ok(())
    }
}
