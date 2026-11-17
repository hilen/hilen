use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{
        Color, Container, Label, ScrollView, Setup, TextAlignment, TextField, ViewData, ViewFrame,
        ViewSubviews, ViewTest, view,
    },
    ui_test::{checkpoint, inject_scroll, inject_touches},
};

const POINTER: Color = Color::rgb(0.85, 0.10, 0.10);

/// A page that scrolls, with 2 multiline text fields on it, like a form. The
/// wheel over the empty field has nothing to scroll there, so it scrolls the
/// page. The wheel over the field with more lines than it shows scrolls that
/// field, and the page stays.
#[view]
struct WheelThroughField {
    pointer: Weak<Container>,
    empty:   Weak<TextField>,
    long:    Weak<TextField>,

    #[init]
    page: ScrollView,
}

impl Setup for WheelThroughField {
    fn setup(mut self: Weak<Self>) {
        self.page.set_color(Color::rgb(0.85, 0.90, 0.95));
        self.page.place().back();

        let title = self.page.add_view::<Label>();
        title.set_text_size(18).set_alignment(TextAlignment::Left);
        title.set_text("a page that scrolls, with 2 text fields");
        title.set_frame((20, 20, 560, 26));

        let empty = self.page.add_view::<TextField>();
        empty.set_multiline(true);
        empty.set_placeholder("empty field, the wheel scrolls the page");
        empty.set_frame((20, 60, 400, 160));
        self.empty = empty;

        let long = self.page.add_view::<TextField>();
        long.set_multiline(true);
        long.set_text(
            (1..=30)
                .map(|line| format!("line {line} of the long field"))
                .collect::<Vec<_>>()
                .join("\n"),
        );
        long.set_frame((20, 260, 400, 160));
        self.long = long;

        let filler = self.page.add_view::<Container>();
        filler.set_frame((0, 1500, 1, 1));

        // The engine's own touch marks take this z, set before the mark is
        // added. It takes no touch.
        let pointer = Container::new();
        pointer.set_z_position(0.1);
        pointer.set_color(POINTER).set_corner_radius(5);
        pointer.set_frame((560, 560, 10, 10));
        self.pointer = self.add_subview(pointer);
    }
}

/// Moves the pointer and the red mark that shows where it is.
fn point(view: Weak<WheelThroughField>, x: f32, y: f32) {
    inject_touches(format!("{x} {y} m"));
    from_main(move || {
        view.pointer.set_frame((x - 5.0, y - 5.0, 10.0, 10.0));
    });
    wait_for_next_frame();
}

impl ViewTest for WheelThroughField {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        // The long field, its text has more lines than it shows: the field
        // scrolls and the page stays.
        point(view, 200.0, 340.0);
        inject_scroll(-120);
        wait_for_next_frame();
        from_main(move || {
            ensure!(
                view.long.content_height() > view.long.height(),
                "the long field shows all of its lines"
            );
            ensure!(
                view.long.scroll_offset() < 0.0,
                "the wheel did not scroll the long field"
            );
            let page = view.page.get_scroll_content_offset();
            ensure!(
                page.abs() < f32::EPSILON,
                "the wheel over the long field moved the page to {page}"
            );
            Ok(())
        })?;
        checkpoint("the wheel over the long field scrolls its lines, the page stays")?;

        // The empty field has nothing to scroll, so the page takes the wheel.
        point(view, 200.0, 140.0);
        inject_scroll(-120);
        wait_for_next_frame();
        from_main(move || {
            let page = view.page.get_scroll_content_offset();
            ensure!(
                page < 0.0,
                "the wheel over the empty field left the page at {page}"
            );
            Ok(())
        })?;
        checkpoint("the wheel over the empty field scrolls the page")?;

        point(view, 560.0, 560.0);
        Ok(())
    }
}
