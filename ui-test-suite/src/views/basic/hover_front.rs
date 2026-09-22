use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{
        BLACK, BLUE, Color, Container, GRAY, GREEN, Hover, Label, RED, ScrollView, Setup, ViewData,
        ViewFrame, ViewSubviews, ViewTest, ViewTouch, WHITE, YELLOW, view,
    },
    ui_test::{checkpoint, inject_touches},
};

/// Hover goes to the view drawn in front under the cursor. A button
/// inside a hoverable row gets the hover, not the row around it. A row
/// clipped by a scroll view is not drawn below the scroll's edge, so a
/// cursor on a bar there does not hover the hidden part of the row.
///
/// A hovered row turns green, the hovered button turns yellow. A pointer
/// move draws nothing on its own, so a black square marks the cursor.
#[view]
struct HoverFront {
    hovered: Option<&'static str>,
    pointer: Weak<Container>,

    #[init]
    row:    Container,
    scroll: ScrollView,
    bar:    Container,
}

impl Setup for HoverFront {
    fn setup(mut self: Weak<Self>) {
        self.row.place().tl(20).size(560, 100);
        caption(self.row, "row", WHITE).place().back();
        let button = self.row.add_view::<Container>();
        button.place().t(20).r(20).size(160, 60);
        caption(button, "button", BLACK).place().back();

        // The row registers after its button, the way a parent's setup
        // runs after its children's. Registration order alone would hand
        // the button's hover to the row.
        self.track(button, "button", RED, YELLOW);
        self.track(self.row, "row", BLUE, GREEN);

        // The scroll shows 160..320 on screen. Its row spans 240..400,
        // so the scroll's bottom edge cuts the row in half.
        self.scroll.set_color(GRAY);
        self.scroll.place().t(160).l(20).size(560, 160);
        let clipped = self.scroll.add_view::<Container>();
        clipped.place().t(80).l(0).size(560, 160);
        caption(clipped, "clipped row, its lower half is hidden", WHITE)
            .place()
            .t(0)
            .lr(0)
            .h(80);
        self.track(clipped, "clipped", BLUE, GREEN);
        let filler = self.scroll.add_view::<Container>();
        filler.place().t(400).l(0).size(1, 1);

        self.bar.set_color(Color::rgb(0.3, 0.3, 0.3));
        self.bar.place().t(320).l(20).size(560, 80);
        caption(
            self.bar,
            "bar, the hidden half of the clipped row is under it",
            WHITE,
        )
        .place()
        .back();

        // A nested view draws over a later sibling, so the marker takes the
        // z of the engine's own touch marks, set before it is added. It is
        // not a hover view, so it never takes the hover itself.
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

fn caption(host: Weak<Container>, text: &str, color: Color) -> Weak<Label> {
    let label = host.add_view::<Label>();
    label.set_text(text).set_text_size(20).set_text_color(color);
    label
}

impl HoverFront {
    fn track(self: Weak<Self>, view: Weak<Container>, name: &'static str, idle: Color, lit: Color) {
        view.set_color(idle);
        view.enable_hover();
        view.touch().hovered.val(self, move |hovered| {
            let mut this = self;
            if hovered {
                this.hovered = Some(name);
            } else if this.hovered == Some(name) {
                this.hovered = None;
            }
            view.set_color(if hovered { lit } else { idle });
        });
    }

    fn hovered_at(view: Weak<Self>, x: u32, y: u32, step: &str) -> Result<Option<&'static str>> {
        inject_touches(format!("{x} {y} m"));
        from_main(move || {
            let mut pointer = view.pointer;
            pointer.set_hidden(false);
            pointer.set_center((x, y));
        });
        // Read before the hold. In human mode the real mouse moves during
        // it and can change the hover.
        let hovered = from_main(move || view.hovered);
        checkpoint(step)?;
        Ok(hovered)
    }
}

impl ViewTest for HoverFront {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        // Cursor position and hover state can leak from previous tests.
        from_main(Hover::clear);

        let on_button = Self::hovered_at(view, 500, 70, "pointer on the button, only the button is yellow")?;
        ensure!(
            on_button == Some("button"),
            "over the button {on_button:?} hovered"
        );

        let on_row = Self::hovered_at(view, 150, 70, "pointer on the row, only the row is green")?;
        ensure!(on_row == Some("row"), "over the row {on_row:?} hovered");

        let on_bar = Self::hovered_at(view, 300, 360, "pointer on the bar, nothing is lit")?;
        ensure!(on_bar.is_none(), "over the bar {on_bar:?} hovered");

        let on_visible = Self::hovered_at(
            view,
            300,
            280,
            "pointer on the visible half, the clipped row is green",
        )?;
        ensure!(
            on_visible == Some("clipped"),
            "over the visible half {on_visible:?} hovered"
        );

        from_main(Hover::clear);
        Ok(())
    }
}
