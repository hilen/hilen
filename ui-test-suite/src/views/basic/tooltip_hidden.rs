use anyhow::Result;
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{
        BLUE, Container, GREEN, Hover, Label, ORANGE, Setup, Tooltip, ViewData, ViewFrame, ViewTest,
        ViewTooltip, ViewTouch, view,
    },
    ui_test::{checkpoint, inject_touches, wait_for_tooltip},
};

#[view]
struct Panel {
    #[init]
    row: Container,
}

impl Setup for Panel {
    fn setup(self: Weak<Self>) {
        self.set_color(BLUE);
        self.row.set_color(ORANGE);
        self.row.place().tl(20).size(160, 40);
        self.row.set_tooltip("Row");
    }
}

/// A key can hide the panel the cursor rests on, with no cursor event.
/// The tooltip of the row inside goes on the next frame, and the view
/// now under the cursor takes the hover.
#[view]
struct TooltipHidden {
    #[init]
    behind:      Container,
    panel:       Panel,
    behind_name: Label,
    row_name:    Label,
}

impl Setup for TooltipHidden {
    fn setup(self: Weak<Self>) {
        self.behind.set_color(GREEN);
        self.behind.place().tl(20).size(400, 300);
        self.behind.set_tooltip("Behind");
        self.behind_name
            .set_text("green: behind")
            .set_text_size(16)
            .place()
            .t(330)
            .l(20)
            .size(300, 30);

        self.panel.place().tl(60).size(200, 160);
        self.panel.bump_z_position(0.001);
        self.row_name
            .set_text("orange: row in blue panel")
            .set_text_size(16)
            .place()
            .t(370)
            .l(20)
            .size(300, 30);
    }
}

impl ViewTest for TooltipHidden {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        from_main(Hover::clear);

        inject_touches("150 100 m");
        wait_for_tooltip();
        from_main(move || {
            assert!(Tooltip::shown().is_ok());
            assert!(view.panel.row.is_hover_within());
        });
        checkpoint("pointer on the row, the row tooltip shows")?;

        from_main(move || {
            view.panel.set_hidden(true);
        });
        wait_for_next_frame();
        from_main(move || {
            assert!(Tooltip::shown().is_null());
            assert!(!view.panel.row.is_hover_within());
            assert!(view.behind.is_hover_within());
        });
        checkpoint("panel hidden, no tooltip")?;

        wait_for_tooltip();
        from_main(|| assert!(Tooltip::shown().is_ok()));
        checkpoint("the green view behind shows its own tooltip")?;

        inject_touches("500 500 m");
        from_main(|| assert!(Tooltip::shown().is_null()));

        Ok(())
    }
}
