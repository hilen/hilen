use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{
        BLACK, BLUE, Container, GRAY, GREEN, Hover, Label, ScrollView, Setup, Tooltip, ViewData, ViewFrame,
        ViewSubviews, ViewTest, ViewTooltip, ViewTouch, WHITE, view,
    },
    ui_test::{checkpoint, inject_scroll, inject_touches, wait_for_tooltip},
};

const ROW: f32 = 80.0;
const ROWS: usize = 6;
/// Where the cursor rests, on row 1 before the scroll.
const POINTER: (u32, u32) = (150, 60);

/// A scroll moves the rows under a still cursor. The row that comes under
/// it takes the hover, the row that left loses it, and the tooltip of the
/// row that left goes at once.
///
/// A hovered row turns green. A pointer move draws nothing on its own, so a
/// black square marks the cursor.
#[view]
struct HoverScroll {
    hovered: Option<usize>,

    #[init]
    scroll: ScrollView,
}

impl Setup for HoverScroll {
    fn setup(self: Weak<Self>) {
        self.scroll.set_color(GRAY);
        self.scroll.place().tl(20).size(560, 240);
        let mut top = 0.0;
        for index in 0..ROWS {
            let row = self.scroll.add_view::<Container>();
            row.place().t(top).l(0).size(560, ROW - 4.0);
            top += ROW;
            let label = row.add_view::<Label>();
            label
                .set_text(format!("row {}", index + 1))
                .set_text_size(20)
                .set_text_color(WHITE);
            label.place().back();
            row.set_tooltip(format!("tooltip of row {}", index + 1));
            self.track(row, index);
        }

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
        let mut pointer = self.add_subview(pointer);
        pointer.set_center(POINTER);
    }
}

impl HoverScroll {
    fn track(self: Weak<Self>, row: Weak<Container>, index: usize) {
        row.set_color(BLUE);
        row.enable_hover();
        row.touch().hovered.val(self, move |hovered| {
            let mut this = self;
            if hovered {
                this.hovered = Some(index);
            } else if this.hovered == Some(index) {
                this.hovered = None;
            }
            row.set_color(if hovered { GREEN } else { BLUE });
        });
    }
}

impl ViewTest for HoverScroll {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        // Cursor position and hover state can leak from previous tests.
        from_main(Hover::clear);

        let (x, y) = POINTER;
        inject_touches(format!("{x} {y} m"));
        wait_for_tooltip();
        // Read before the hold. In human mode the real mouse moves during it
        // and can change the hover.
        let (hovered, tooltip) = from_main(move || (view.hovered, Tooltip::shown().is_ok()));
        ensure!(hovered == Some(0), "before the scroll {hovered:?} hovered");
        ensure!(tooltip, "the tooltip of row 1 did not show");
        checkpoint("pointer on row 1, row 1 is green, its tooltip shows")?;

        // 1 row up, row 2 comes under the cursor.
        inject_scroll(-ROW);
        wait_for_next_frame();
        wait_for_next_frame();
        let (hovered, tooltip) = from_main(move || (view.hovered, Tooltip::shown().is_ok()));
        ensure!(hovered == Some(1), "after the scroll {hovered:?} hovered");
        ensure!(!tooltip, "the tooltip of row 1 stayed after the scroll");
        checkpoint("row 2 scrolled under the pointer, row 2 is green, no tooltip")?;

        // Resting on row 2 shows its own tooltip.
        wait_for_tooltip();
        let tooltip = from_main(|| Tooltip::shown().is_ok());
        ensure!(tooltip, "the tooltip of row 2 did not show");
        checkpoint("the tooltip of row 2 shows")?;

        from_main(Hover::clear);
        Ok(())
    }
}
