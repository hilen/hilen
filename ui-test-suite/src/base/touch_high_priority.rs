use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    gm::color::{GRAY, LIGHT_GRAY},
    refs::Weak,
    ui::{
        Button, Color, Container, Label, Setup, ViewData, ViewFrame, ViewSubviews, ViewTest, ViewTouch, view,
    },
    ui_test::{checkpoint, inject_touches},
};

const ROW_HEIGHT: f32 = 60.0;
const ROWS: u8 = 8;

/// A list with 2 buttons that float over it, the way a mic floats over
/// a chat. The rows come after the buttons and turn their touch on
/// later, like the rows of a reply that grows. The blue button turned
/// high priority on and takes its tap. The red one is a plain touch view,
/// the row under it takes its tap, which shows that the rows do win the
/// plain order. The text at the bottom names the view that took the
/// last tap.
#[view]
struct TouchHighPriority {
    tapped: String,

    #[init]
    list:   Container,
    high:   Button,
    plain:  Button,
    status: Label,
}

impl TouchHighPriority {
    fn set_tapped(mut self: Weak<Self>, name: &str) {
        self.tapped = name.to_string();
        self.status.set_text(format!("tapped: {name}"));
    }

    fn tap(self: Weak<Self>, x: u32, y: u32) -> String {
        inject_touches(format!("{x} {y} b\n{x} {y} e"));
        from_main(move || self.tapped.clone())
    }

    fn add_rows(self: Weak<Self>) {
        for index in 0..ROWS {
            let row = self.list.add_view::<Button>();
            let name = format!("row {}", index + 1);
            row.set_text(&name).set_text_size(20);
            row.set_color(if index % 2 == 0 { LIGHT_GRAY } else { GRAY });
            row.set_frame((0.0, f32::from(index) * ROW_HEIGHT, 520.0, ROW_HEIGHT));
            row.on_tap(move || self.set_tapped(&name));
        }
    }
}

impl Setup for TouchHighPriority {
    fn setup(self: Weak<Self>) {
        self.list.set_frame((40, 40, 520, 480));

        self.plain.set_text("plain").set_text_size(20);
        self.plain.set_color(Color::rgb(0.9, 0.3, 0.3));
        self.plain.set_frame((400, 170, 140, 40));
        self.plain.bump_z_position(0.000_1);
        self.plain.on_tap(move || self.set_tapped("plain"));

        self.high.set_text("high").set_text_size(20);
        self.high.set_color(Color::rgb(0.3, 0.5, 1.0));
        self.high.set_frame((400, 290, 140, 40));
        self.high.bump_z_position(0.000_1);
        self.high.enable_touch_high_priority();
        self.high.on_tap(move || self.set_tapped("high"));

        self.status.set_text_size(22);
        self.status.set_frame((40, 540, 520, 40));
        self.set_tapped("none");
    }
}

impl ViewTest for TouchHighPriority {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        from_main(move || view.add_rows());
        checkpoint("rows added after the buttons, both buttons drawn over the rows")?;

        let tapped = view.tap(470, 310);
        ensure!(tapped == "high", "a tap on the high button reached {tapped}");
        checkpoint("tap on the blue high button, the high button took it")?;

        let tapped = view.tap(470, 190);
        ensure!(tapped == "row 3", "a tap on the plain button reached {tapped}");
        checkpoint("tap on the red plain button, row 3 under it took it")?;

        let tapped = view.tap(150, 310);
        ensure!(tapped == "row 5", "a tap beside the high button reached {tapped}");
        checkpoint("tap on row 5 beside the high button, the row took it")?;

        Ok(())
    }
}
