use anyhow::Result;
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{Button, Label, MouseButton, Setup, UIEvents, ViewFrame, ViewTest, view},
    ui_test::{checkpoint, inject_click, inject_touches},
};

/// The Back, Forward and middle buttons of a mouse press no view. A Back
/// press over a button tapped it before. The app still hears every
/// button through `UIEvents::touch_began`.
#[view]
struct SideMouseButtons {
    taps:  usize,
    heard: Vec<MouseButton>,

    #[init]
    title:  Label,
    button: Button,
    state:  Label,
}

impl SideMouseButtons {
    fn paint(self: Weak<Self>) {
        self.button.set_text(format!("button, taps: {}", self.taps));
        let heard: Vec<String> = self.heard.iter().map(|button| format!("{button:?}")).collect();
        self.state.set_text(format!("the app heard: {}", heard.join(", ")));
    }
}

impl Setup for SideMouseButtons {
    fn setup(self: Weak<Self>) {
        self.title.set_frame((20, 20, 560, 30));
        self.title.set_text_size(24).set_text("side buttons of a mouse over a button");

        self.button.set_frame((100, 100, 400, 120));
        self.button.on_tap(move || {
            let mut this = self;
            this.taps += 1;
            this.paint();
        });

        self.state.set_frame((20, 260, 560, 30));
        self.state.set_text_size(24);

        UIEvents::touch_began().val(self, move |touch| {
            let mut this = self;
            this.heard.push(touch.button);
            this.paint();
        });
        self.paint();
    }
}

impl ViewTest for SideMouseButtons {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        for button in [MouseButton::Back, MouseButton::Forward, MouseButton::Middle] {
            inject_click(300, 160, button);
        }
        from_main(move || {
            assert_eq!(view.taps, 0, "a side button tapped the button");
            assert_eq!(
                view.heard,
                [MouseButton::Back, MouseButton::Forward, MouseButton::Middle]
            );
        });
        checkpoint("Back, Forward and middle heard, the button has 0 taps")?;

        // The left button still taps.
        inject_touches("300 160 b\n300 160 e");
        from_main(move || {
            assert_eq!(view.taps, 1);
            assert_eq!(view.heard.len(), 4);
        });
        checkpoint("a left click tapped the button once")?;

        Ok(())
    }
}
