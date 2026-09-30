use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{
        BLACK, Color, Container, Hover, Label, Setup, ViewData, ViewFrame, ViewSubviews, ViewTest, ViewTouch,
        WHITE, YELLOW, view,
    },
    ui_test::{checkpoint, inject_touches},
};

const IDLE: Color = Color::rgb(0.25, 0.28, 0.33);
const WASH: Color = Color::rgb(0.35, 0.5, 0.7);
const BUTTON: Color = Color::rgb(0.9, 0.45, 0.1);

/// A row knows the pointer is over it or over anything inside it. Each
/// row shows its Add button and a lighter wash only while hover is
/// within the row, so the button stays up while the pointer is on it.
///
/// A pointer move draws nothing on its own, so a black square marks the
/// cursor. The log under the rows lists every enter and exit a row got.
#[view]
struct HoverWithin {
    log:     Vec<String>,
    pointer: Weak<Container>,

    #[init]
    first:   Container,
    second:  Container,
    caption: Label,
}

impl Setup for HoverWithin {
    fn setup(mut self: Weak<Self>) {
        self.first.place().t(40).lr(20).h(100);
        self.second.place().t(160).lr(20).h(100);
        self.add_row(self.first, "first row");
        self.add_row(self.second, "second row");

        self.caption.set_text_size(20).set_text_color(BLACK);
        self.caption.place().t(300).lr(20).h(40);

        let mut pointer = Container::new();
        pointer.set_z_position(0.1);
        pointer
            .set_color(BLACK)
            .set_border_width(3)
            .set_border_color(WHITE)
            .set_size(20, 20);
        pointer.set_hidden(true);
        self.pointer = self.add_subview(pointer);
    }
}

impl HoverWithin {
    fn add_row(self: Weak<Self>, row: Weak<Container>, name: &'static str) {
        row.set_color(IDLE);
        row.enable_hover();
        let label = row.add_view::<Label>();
        label.set_text(name).set_text_size(20).set_text_color(WHITE);
        label.place().l(20).t(0).b(0).w(200);

        let button = row.add_view::<Container>();
        button.set_color(BUTTON);
        button.place().t(20).r(20).size(160, 60);
        let add = button.add_view::<Label>();
        add.set_text("Add").set_text_size(20).set_text_color(BLACK);
        add.place().back();
        button.enable_hover();
        button.touch().hovered.val(self, move |hovered| {
            button.set_color(if hovered { YELLOW } else { BUTTON });
        });
        button.set_hidden(true);

        row.touch().hover_within.val(self, move |within| {
            let mut this = self;
            this.log.push(format!("{name} {}", if within { "in" } else { "out" }));
            let text = this.log.join(", ");
            this.caption.set_text(text);
            button.set_hidden(!within);
            row.set_color(if within { WASH } else { IDLE });
        });
    }

    fn move_to(view: Weak<Self>, x: u32, y: u32, step: &str) -> Result<String> {
        inject_touches(format!("{x} {y} m"));
        from_main(move || {
            let mut pointer = view.pointer;
            pointer.set_hidden(false);
            pointer.set_center((x, y));
        });
        // Read before the hold. In human mode the real mouse moves during
        // it and can change the hover.
        let log = from_main(move || view.log.join(", "));
        checkpoint(step)?;
        Ok(log)
    }
}

impl ViewTest for HoverWithin {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        from_main(Hover::clear);
        from_main(move || {
            let mut view = view;
            view.log.clear();
            view.caption.set_text("");
        });

        let log = Self::move_to(
            view,
            150,
            90,
            "pointer on the first row, it lights up and shows Add",
        )?;
        ensure!(log == "first row in", "on the first row: {log}");

        let log = Self::move_to(
            view,
            480,
            90,
            "pointer on Add, the row stays lit and Add turns yellow",
        )?;
        ensure!(log == "first row in", "on the first row button: {log}");
        ensure!(from_main(move || view.first.is_hover_within()));

        let log = Self::move_to(
            view,
            150,
            90,
            "pointer back on the row, nothing changes on the row",
        )?;
        ensure!(log == "first row in", "back on the first row: {log}");

        Self::move_to(view, 480, 90, "pointer on Add of the first row again")?;
        let log = Self::move_to(
            view,
            480,
            210,
            "pointer on the second row, the first goes dark, the second lights up",
        )?;
        ensure!(
            log == "first row in, first row out, second row in",
            "from the first button to the second row, the exit first: {log}"
        );

        let log = Self::move_to(view, 300, 450, "pointer below the rows, both are dark, no Add")?;
        ensure!(
            log == "first row in, first row out, second row in, second row out",
            "off the rows: {log}"
        );
        ensure!(from_main(
            move || !view.first.is_hover_within() && !view.second.is_hover_within()
        ));

        from_main(Hover::clear);
        Ok(())
    }
}
