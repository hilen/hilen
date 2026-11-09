use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{
        Button, Focus, Label, Modifiers, NamedKey, Setup, View, ViewData, ViewFrame, ViewTest, WeakView, view,
    },
    ui_test::{checkpoint, inject_modifiers, inject_named_key},
};

/// Tab moves the focus ring to the next control in reading order, Shift Tab
/// to the one before. The order goes by lines from the top and in a line
/// from the left, so 3 buttons of different heights side by side are 1
/// line. With no scroll view the ring goes round at the end.
#[view]
struct KeyFocusTab {
    #[init]
    title: Label,
    a:     Button,
    b:     Button,
    c:     Button,
    wide:  Button,
    e:     Button,
    f:     Button,
    state: Label,
}

impl KeyFocusTab {
    fn buttons(self: Weak<Self>) -> [(Weak<Button>, &'static str); 6] {
        [
            (self.a, "a"),
            (self.b, "b"),
            (self.c, "c"),
            (self.wide, "wide"),
            (self.e, "e"),
            (self.f, "f"),
        ]
    }

    fn name_of(self: Weak<Self>, view: WeakView) -> &'static str {
        if view.is_null() {
            return "none";
        }
        self.buttons()
            .into_iter()
            .find(|(button, _)| button.weak_view().raw() == view.raw())
            .map_or("unknown", |(_, name)| name)
    }

    fn tab(self: Weak<Self>, shift: bool, expected: &str) -> Result<()> {
        if shift {
            inject_modifiers(Modifiers::SHIFT);
        }
        inject_named_key(NamedKey::Tab);
        if shift {
            inject_modifiers(Modifiers::empty());
        }
        wait_for_next_frame();
        let focused = from_main(move || {
            let name = self.name_of(Focus::focused());
            self.state.set_text(format!("ring: {name}"));
            name
        });
        let key = if shift { "Shift Tab" } else { "Tab" };
        ensure!(
            focused == expected,
            "after {key} the ring is on {focused}, expected {expected}"
        );
        wait_for_next_frame();
        Ok(())
    }
}

impl Setup for KeyFocusTab {
    fn setup(self: Weak<Self>) {
        self.title.set_text("Tab goes on in reading order").set_text_size(20);
        self.title.place().lrt(10).h(40);

        // 1 line of 3 buttons of different heights, a wide one under it,
        // then a line of 2.
        let frames: [(usize, usize, usize, usize); 6] = [
            (40, 100, 150, 40),
            (220, 90, 150, 70),
            (400, 110, 150, 50),
            (40, 220, 510, 60),
            (40, 320, 240, 60),
            (310, 320, 240, 60),
        ];
        for ((button, name), frame) in self.buttons().into_iter().zip(frames) {
            button.set_text(name).set_text_size(22);
            button.set_color("#cfd8e3").set_corner_radius(10);
            button.set_frame(frame);
            button.on_tap(|| {});
        }

        self.state.set_text("ring: none").set_text_size(20);
        self.state.place().lrb(10).h(40);
    }
}

impl ViewTest for KeyFocusTab {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        from_main(Focus::hide);

        view.tab(false, "a")?;
        checkpoint("the first Tab shows the ring on a")?;

        view.tab(false, "b")?;
        view.tab(false, "c")?;
        checkpoint("a, b and c are 1 line, the ring is on c")?;

        view.tab(false, "wide")?;
        view.tab(false, "e")?;
        view.tab(false, "f")?;
        checkpoint("the ring is on f, the last one")?;

        view.tab(false, "a")?;
        checkpoint("Tab after the last one goes round to a")?;

        view.tab(true, "f")?;
        view.tab(true, "e")?;
        view.tab(true, "wide")?;
        checkpoint("Shift Tab goes back, the ring is on wide")?;

        from_main(Focus::hide);
        Ok(())
    }
}
