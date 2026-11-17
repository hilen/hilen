use anyhow::Result;
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{Anchor, Label, Setup, Slider, ViewData, ViewTest, WHITE, view},
    ui_test::{checkpoint, inject_touches},
};

/// A slider whose owner counts the drags: the label names how many began
/// and ended and how many values came, so an owner can keep 1 undo step
/// per drag.
#[view]
struct SliderDrag {
    began:   u32,
    ended:   u32,
    changes: u32,

    #[init]
    slider: Slider,
    status: Label,
}

impl SliderDrag {
    fn show_status(&self) {
        self.status.set_text(format!(
            "slider: began {}, ended {}, values {}",
            self.began, self.ended, self.changes
        ));
    }
}

impl Setup for SliderDrag {
    fn setup(mut self: Weak<Self>) {
        self.slider.set_horizontal().set_color(WHITE).place().size(400, 50).center();
        self.slider.on_change.val(move |_| {
            self.changes += 1;
            self.show_status();
        });
        self.slider.drag_began.sub(move || {
            self.began += 1;
            self.show_status();
        });
        self.slider.drag_ended.sub(move || {
            self.ended += 1;
            self.show_status();
        });

        self.status.set_text_size(18).set_color(WHITE);
        self.status
            .place()
            .size(400, 40)
            .center_x()
            .anchor(Anchor::Top, self.slider, 30);
    }
}

impl ViewTest for SliderDrag {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        // Setting the range in setup reports values, they are no drag.
        let before = from_main(move || {
            assert_eq!((view.began, view.ended), (0, 0));
            assert!(!view.slider.is_dragging());
            view.changes
        });

        inject_touches(
            r"
                393  300  b
                350  302  m
                260  305  m
        ",
        );
        from_main(move || {
            assert_eq!((view.began, view.ended), (1, 0));
            assert!(view.slider.is_dragging());
            assert_eq!(view.changes, before + 3);
        });
        checkpoint("the thumb is held: began 1, ended 0")?;

        inject_touches("207  301  e");
        from_main(move || {
            assert_eq!((view.began, view.ended), (1, 1));
            assert!(!view.slider.is_dragging());
        });
        checkpoint("the drag ended: began 1, ended 1")?;

        // A tap is a drag of 1 value.
        inject_touches("300 300 b\n300 300 e");
        from_main(move || {
            assert_eq!((view.began, view.ended), (2, 2));
            assert_eq!(view.changes, before + 4);
        });
        checkpoint("a tap: began 2, ended 2")?;

        Ok(())
    }
}
