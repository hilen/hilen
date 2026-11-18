use anyhow::{Result, ensure};

use crate::{
    self as hilen,
    deps::{hreads::from_main, refs::Weak},
    gm::color::{BLACK, GRAY, LIGHT_BLUE, WHITE},
    ui::{
        Color, Container, CursorIcon, Hover, Label, Setup, VSplitView, ViewData, ViewFrame, ViewSubviews,
        ViewTest, view,
    },
    ui_test::{checkpoint, inject_touches},
};

/// The divider of a `VSplitView` shows the up and down resize cursor, and
/// the cursor survives the whole drag. A drag outruns the thin handle
/// between frames, and without the hover lock every such move would pick
/// the view under the cursor again and drop the cursor icon.
///
/// A pointer move draws nothing on its own, so a black square marks the
/// cursor, and the status line names the cursor icon the app asks for.
#[view]
struct VSplitHoverLock {
    pointer: Weak<Container>,
    status:  Weak<Label>,

    #[init]
    split: VSplitView,
}

impl Setup for VSplitHoverLock {
    fn setup(mut self: Weak<Self>) {
        self.split.place().back();
        self.split.set_min_heights(200.0, 100.0);
        self.split.set_bottom_height(150.0);
        self.split.set_divider_colors(GRAY, LIGHT_BLUE);

        self.split.top.set_color(Color::hex("#DCE6FB"));
        self.split.bottom.set_color(Color::hex("#FBE9D0"));

        let top = self.split.top.add_view::<Label>();
        top.set_text("top panel").set_text_size(20);
        top.place().tl(10).size(300, 30);

        self.status = self.split.top.add_view();
        self.status.set_text_size(20);
        self.status.place().t(50).l(10).size(500, 30);

        let bottom = self.split.bottom.add_view::<Label>();
        bottom.set_text("bottom panel").set_text_size(20);
        bottom.place().bl(10).size(300, 30);

        // A nested view draws over a later sibling, so the marker takes the
        // z of the engine's own touch marks, set before it is added. It is
        // not a hover view, so it never takes the hover itself.
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

impl VSplitHoverLock {
    /// Sends the touches, the last of them at `x` `y`, and gives the
    /// cursor icon the app asks for after them.
    fn cursor_after(
        view: Weak<Self>,
        touches: &'static str,
        x: u32,
        y: u32,
        step: &str,
    ) -> Result<CursorIcon> {
        inject_touches(touches);
        Self::cursor_now(view, x, y, step)
    }

    fn cursor_now(view: Weak<Self>, x: u32, y: u32, step: &str) -> Result<CursorIcon> {
        // Read before the hold. In human mode the real mouse moves during
        // it and can change the hover.
        let cursor = from_main(move || {
            let cursor = Hover::cursor();
            view.pointer.set_hidden(false);
            view.pointer.set_center((x, y));
            view.status.set_text(format!("cursor {cursor:?}"));
            cursor
        });
        checkpoint(step)?;
        Ok(cursor)
    }
}

impl ViewTest for VSplitHoverLock {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        // A headed window can open under the real mouse, whose entry
        // event hovers whatever it lands on before the first injection.
        from_main(Hover::clear);

        let cursor = Self::cursor_after(
            view,
            "300 300 m",
            300,
            300,
            "pointer on the top panel, a plain cursor",
        )?;
        ensure!(
            cursor == CursorIcon::Default,
            "on the top panel the cursor is {cursor:?}"
        );

        let cursor = Self::cursor_after(
            view,
            "300 450 m",
            300,
            450,
            "pointer on the divider, the resize cursor",
        )?;
        ensure!(
            cursor == CursorIcon::RowResize,
            "on the divider the cursor is {cursor:?}"
        );

        // Mid drag the cursor is far off the handle, the icon holds. The
        // bottom panel stops at the top minimum, 600 - 200.
        let cursor = Self::cursor_after(
            view,
            "
            300 450 b
            300 100 m
        ",
            300,
            100,
            "dragged far above the divider, still the resize cursor",
        )?;
        ensure!(
            cursor == CursorIcon::RowResize,
            "mid drag the cursor is {cursor:?}"
        );

        // The cursor leaving the window mid drag must not drop it either.
        from_main(Hover::clear);
        let cursor = Self::cursor_now(
            view,
            300,
            100,
            "the pointer left the window mid drag, still the resize cursor",
        )?;
        ensure!(
            cursor == CursorIcon::RowResize,
            "out of the window mid drag the cursor is {cursor:?}"
        );

        // The release picks the view under the real cursor again.
        let cursor = Self::cursor_after(
            view,
            "300 100 e",
            300,
            100,
            "released on the top panel, a plain cursor",
        )?;
        ensure!(
            cursor == CursorIcon::Default,
            "after the release the cursor is {cursor:?}"
        );

        let cursor = Self::cursor_after(
            view,
            "300 200 m",
            300,
            200,
            "pointer on the moved divider, the resize cursor",
        )?;
        ensure!(
            cursor == CursorIcon::RowResize,
            "on the moved divider the cursor is {cursor:?}"
        );

        Ok(())
    }
}
