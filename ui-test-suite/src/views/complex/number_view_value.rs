use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{Color, Label, NumberView, Setup, TextAlignment, ViewData, ViewFrame, ViewTest, WHITE, view},
    ui_test::{check_colors, checkpoint, inject_touches},
};

const BORDER: Color = Color::rgb(0.80, 0.84, 0.90);
const TEXT: Color = Color::rgb(0.10, 0.12, 0.17);

/// The stepper at its maximum, 12 left of the arrows, and the status.
const AT_MAX: &str = r"
    592    4 - #ffffff
    124  100 - #ccd6e6
    148  100 - #ccd6e6
    168  100 - #ccd6e6
    188  100 - #ccd6e6
    204  100 - #ccd6e6
    220  100 - #ccd6e6
    240  100 - #ccd6e6
    100  112 - #ccd6e6
    252  112 - #ffffff
    440  124 - #ffffff
    124  128 - #ffffff
    100  132 - #ccd6e6
    128  136 - #484c56
    248  140 - #ffffff
    100  148 - #ccd6e6
    592  164 - #ffffff
    116  216 - #000000
    48  220 - #ffffff
    68  220 - #000000
    72  220 - #ffffff
    100  220 - #000000
    104  220 - #1e1e1e
    116  220 - #000000
    508  300 - #ffffff
    132  372 - #ffffff
    288  376 - #ffffff
    592  432 - #ffffff
    4  456 - #ffffff
    400  540 - #ffffff
    204  592 - #ffffff
    592  592 - #ffffff
";

/// A stepper that shows its value, like a text size on a settings page:
/// from 9 to 12 by 1, starting at 10. The value stands left of the arrows.
/// 2 taps up reach 12, a 3rd tap stays at 12, a tap down goes to 11.
#[view]
struct NumberViewValue {
    #[init]
    stepper: NumberView,
    status:  Label,
}

impl Setup for NumberViewValue {
    fn setup(self: Weak<Self>) {
        self.set_color(WHITE);
        self.stepper.show_value().set_chevrons().set_separator_color(BORDER);
        self.stepper.set_text_size(20).set_text_color(TEXT);
        self.stepper.set_min(9).set_max(12).set_step(1).set_value(10);
        self.stepper.set_corner_radius(8).set_border_width(1).set_border_color(BORDER);
        self.stepper.place().t(100).l(100).size(160, 60);
        self.stepper.on_change(move |value| {
            self.status.set_text(format!("stepper at {value}"));
        });

        self.status
            .set_text("stepper at 10")
            .set_text_size(16)
            .set_alignment(TextAlignment::Left);
        self.status.place().t(200).lr(20).h(40);
    }
}

impl NumberViewValue {
    /// A tap on the up half of the arrows, or on the down half.
    fn tap(self: Weak<Self>, up: bool) {
        let stepper = self.stepper;
        let (x, y) = from_main(move || {
            let frame = stepper.absolute_frame();
            let y = if up {
                frame.y() + frame.height() / 4.0
            } else {
                frame.y() + frame.height() * 3.0 / 4.0
            };
            (frame.max_x() - 14.0, y)
        });
        inject_touches(format!("{x} {y} b\n{x} {y} e"));
        wait_for_next_frame();
    }

    /// The steps are whole numbers here, so a margin only keeps the float
    /// comparison honest.
    fn is_at(self: Weak<Self>, expected: f32) -> bool {
        (self.value() - expected).abs() < f32::EPSILON
    }

    fn value(self: Weak<Self>) -> f32 {
        let stepper = self.stepper;
        from_main(move || stepper.value())
    }
}

impl ViewTest for NumberViewValue {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        view.tap(true);
        view.tap(true);
        ensure!(view.is_at(12.0), "2 taps up gave {}", view.value());
        view.tap(true);
        ensure!(view.is_at(12.0), "a tap up at the maximum gave {}", view.value());
        // The value 12 stands left of the arrows, the status says it too.
        check_colors(AT_MAX)?;

        view.tap(false);
        ensure!(view.is_at(11.0), "a tap down gave {}", view.value());
        checkpoint("a tap down: the stepper shows 11")?;
        Ok(())
    }
}
