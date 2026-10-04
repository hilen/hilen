use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{
        Button, Container, Focus, FocusDirection, Label, NamedKey, Setup, View, ViewData, ViewFocus,
        ViewFrame, ViewTest, ViewTouch, WeakView, view,
    },
    ui_test::{check_colors, checkpoint, inject_named_key, inject_touches},
};

/// The arrow keys move the focus ring between the views that take a tap,
/// Enter taps the view under it, and a pointer takes over from the keys.
#[view]
struct KeyFocus {
    tapped: String,

    #[init]
    title:    Label,
    backdrop: Container,
    b1:       Button,
    b2:       Button,
    b3:       Button,
    b4:       Button,
    b5:       Button,
    b6:       Button,
    far:      Button,
    state:    Label,
}

impl KeyFocus {
    fn buttons(self: Weak<Self>) -> [Weak<Button>; 6] {
        [self.b1, self.b2, self.b3, self.b4, self.b5, self.b6]
    }

    fn name_of(self: Weak<Self>, view: WeakView) -> String {
        if view.is_null() {
            return "none".into();
        }
        if view.raw() == self.far.weak_view().raw() {
            return "far".into();
        }
        if view.raw() == self.backdrop.weak_view().raw() {
            return "backdrop".into();
        }
        self.buttons()
            .iter()
            .position(|button| button.weak_view().raw() == view.raw())
            .map_or_else(|| "unknown".into(), |index| (index + 1).to_string())
    }

    fn focused(self: Weak<Self>) -> String {
        from_main(move || {
            let name = self.name_of(Focus::focused());
            self.state.set_text(format!("ring: {name}, tapped: {}", self.tapped));
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
}

impl Setup for KeyFocus {
    fn setup(mut self: Weak<Self>) {
        self.title.set_text("arrows move the ring, Enter taps").set_text_size(20);
        self.title.place().lrt(10).h(40);

        // Takes touches over the whole canvas and is kept out of the focus.
        self.backdrop.set_color("#eef1f5");
        self.backdrop.place().lr(0).t(60).b(60);
        self.backdrop.enable_touch_low_priority();
        self.backdrop.set_key_focus(false);

        for (index, button) in self.buttons().into_iter().enumerate() {
            let column = index % 3;
            let row = index / 3;
            button.set_text(index + 1).set_text_size(24);
            button.set_color("#cfd8e3").set_corner_radius(10);
            button.set_frame((40 + column * 180, 120 + row * 110, 140_usize, 70_usize));
            button.on_tap(move || {
                self.tapped = (index + 1).to_string();
            });
        }

        self.far.set_text("far, up goes to 1").set_text_size(18);
        self.far.set_color("#f0d060").set_corner_radius(10);
        self.far.set_frame((330, 440, 230, 70));
        self.far.on_tap(move || {
            self.tapped = "far".into();
        });
        // The nearest view above is 6, the app names 1 instead.
        self.far.set_focus_neighbor(FocusDirection::Up, self.b1.weak_view());

        self.tapped = "nothing".into();
        self.state.set_text_size(20);
        self.state.place().lrb(10).h(40);
    }
}

impl ViewTest for KeyFocus {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        ensure!(view.focused() == "none", "a ring before any key");

        view.press(NamedKey::ArrowRight, "1")?;
        checkpoint("the first key shows the ring on 1")?;

        view.press(NamedKey::ArrowRight, "2")?;
        check_colors(RING_ON_2)?;

        view.press(NamedKey::ArrowRight, "3")?;
        view.press(NamedKey::ArrowRight, "3")?;
        checkpoint("nothing right of 3, the ring stays")?;

        view.press(NamedKey::ArrowDown, "6")?;
        view.press(NamedKey::ArrowLeft, "5")?;
        view.press(NamedKey::ArrowUp, "2")?;
        view.press(NamedKey::ArrowDown, "5")?;

        inject_named_key(NamedKey::Enter);
        wait_for_next_frame();
        view.focused();
        ensure!(
            from_main(move || view.tapped.clone()) == "5",
            "Enter did not tap 5"
        );
        checkpoint("Enter tapped 5")?;

        view.press(NamedKey::ArrowRight, "6")?;
        view.press(NamedKey::ArrowDown, "far")?;
        checkpoint("down from 6 is the far button")?;

        view.press(NamedKey::ArrowUp, "1")?;
        checkpoint("up from far goes to 1, named by hand")?;

        // A pointer press takes over, the ring goes.
        inject_touches(
            "
            300 30 b
            300 30 e
        ",
        );
        wait_for_next_frame();
        ensure!(view.focused() == "none", "the ring stayed after a pointer press");
        check_colors(NO_RING)?;

        view.press(NamedKey::ArrowLeft, "1")?;
        checkpoint("a key brings the ring back where it was")?;

        Ok(())
    }
}

const RING_ON_2: &str = "";

const NO_RING: &str = "";
