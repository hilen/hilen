use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{
        BLACK, Color, Container, CursorIcon, Hover, Label, Setup, ViewData, ViewFrame, ViewSubviews,
        ViewTest, ViewTouch, WHITE, view,
    },
    ui_test::{checkpoint, inject_touches},
};

const ARROW: Color = Color::rgb(0.25, 0.28, 0.33);
const HAND: Color = Color::rgb(0.2, 0.45, 0.8);

/// A view changes the cursor it asks for while the pointer rests on it,
/// like a terminal that shows a hand only over a link. The new cursor
/// shows at once, with no move of the pointer. Before, it showed only
/// after the pointer had left the view and come back.
///
/// A pointer move draws nothing on its own, so a black square marks the
/// cursor. The area is blue while it asks for the hand, and the caption
/// names the cursor the window shows.
#[view]
struct HoverCursorChange {
    pointer: Weak<Container>,

    #[init]
    area:    Container,
    other:   Container,
    caption: Label,
}

impl Setup for HoverCursorChange {
    fn setup(mut self: Weak<Self>) {
        self.area.place().t(40).lr(20).h(200);
        Self::name(self.area, "area");
        self.other.place().t(280).lr(20).h(100);
        Self::name(self.other, "other view");
        self.other.set_color(ARROW).enable_hover();

        self.caption.set_text_size(20).set_text_color(BLACK);
        self.caption.place().t(420).lr(20).h(40);

        let pointer = Container::new();
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

impl HoverCursorChange {
    fn name(view: Weak<Container>, name: &str) {
        let label = view.add_view::<Label>();
        label.set_text(name).set_text_size(20).set_text_color(WHITE);
        label.place().l(20).t(0).b(0).w(300);
    }

    /// The area asks for `icon`, blue for the hand.
    fn ask(view: Weak<Self>, icon: CursorIcon) {
        from_main(move || {
            view.area.set_hover_cursor(icon);
            view.area.set_color(if icon == CursorIcon::Pointer { HAND } else { ARROW });
        });
    }

    fn move_to(view: Weak<Self>, x: u32, y: u32) {
        inject_touches(format!("{x} {y} m"));
        from_main(move || {
            view.pointer.set_hidden(false);
            view.pointer.set_center((x, y));
        });
    }

    /// The cursor the window shows, written in the caption. Read before
    /// the hold, in human mode the real mouse moves during it.
    fn shown(view: Weak<Self>, step: &str) -> Result<CursorIcon> {
        let cursor = from_main(move || {
            let cursor = Hover::cursor();
            view.caption.set_text(format!("the window shows {cursor:?}"));
            cursor
        });
        checkpoint(step)?;
        Ok(cursor)
    }
}

impl ViewTest for HoverCursorChange {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        from_main(Hover::clear);
        Self::ask(view, CursorIcon::Default);

        Self::move_to(view, 150, 140);
        let cursor = Self::shown(view, "pointer on the gray area, the arrow")?;
        ensure!(cursor == CursorIcon::Default, "on the area: {cursor:?}");

        Self::ask(view, CursorIcon::Pointer);
        let cursor = Self::shown(
            view,
            "the area turns blue and asks for the hand, it shows with no move",
        )?;
        ensure!(
            cursor == CursorIcon::Pointer,
            "the hand under a still pointer: {cursor:?}"
        );

        Self::ask(view, CursorIcon::Default);
        let cursor = Self::shown(view, "the area turns gray again, the arrow is back with no move")?;
        ensure!(
            cursor == CursorIcon::Default,
            "the arrow under a still pointer: {cursor:?}"
        );

        Self::move_to(view, 400, 330);
        Self::ask(view, CursorIcon::Pointer);
        let cursor = Self::shown(
            view,
            "pointer on the other view, the blue area asks for the hand, the arrow stays",
        )?;
        ensure!(
            cursor == CursorIcon::Default,
            "the area is not under the pointer: {cursor:?}"
        );

        Self::move_to(view, 150, 140);
        let cursor = Self::shown(view, "pointer back on the blue area, the hand")?;
        ensure!(cursor == CursorIcon::Pointer, "back on the area: {cursor:?}");

        from_main(Hover::clear);
        Ok(())
    }
}
