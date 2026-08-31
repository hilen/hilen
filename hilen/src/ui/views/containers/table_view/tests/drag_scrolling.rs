use std::ops::Deref;

use anyhow::Result;
use parking_lot::Mutex;

use crate::{
    self as hilen,
    deps::{hreads::from_main, refs::Weak},
    gm::color::Color,
    ui::{CellRegistry, Label, Setup, TableData, TableView, UIManager, View, ViewData, ViewTest, view},
    ui_test::{inject_scroll, inject_touches},
};

static SELECTED: Mutex<String> = Mutex::new(String::new());

// Drag scrolling is a touch gesture. With `UIManager::set_drag_scrolling`
// off, the desktop default, a mouse drag must leave the table alone so
// the views under it can take it, text selection first of all, and the
// wheel keeps scrolling. Switched on, the finger default of the touch
// platforms, the same drag must scroll.
#[view]
struct TableDragScrolling {
    #[init]
    table: TableView,
}

impl Setup for TableDragScrolling {
    fn setup(mut self: Weak<Self>) {
        self.table.place().tl(0).size(400, 600);
        self.table.set_data_source(self).register_cell::<Label>();
        self.table.reload_data();
    }
}

impl TableData for TableDragScrolling {
    fn cell_height(&self, _: usize) -> f32 {
        90.0
    }

    fn number_of_cells(&self) -> usize {
        100
    }

    fn setup_cell(&mut self, index: usize, registry: &mut CellRegistry) -> Weak<dyn View> {
        let cell = registry.cell::<Label>();
        cell.set_text(format!("Row {index}"));
        cell.set_color(if index.is_multiple_of(2) {
            Color::hex("#dfe8f0")
        } else {
            Color::hex("#c3d2e0")
        });
        cell
    }

    fn cell_selected(&mut self, index: usize) {
        *SELECTED.lock() += &format!("|{index}|");
    }
}

// A slow visible sweep from the middle of the table to its top, so a
// human run can watch whether the rows follow it.
fn drag_up() {
    let mut drag = String::from("200 500 b\n");
    for step in 1..=20 {
        drag += &format!("200 {} m\n", 500 - step * 20);
    }
    drag += "200 100 e";
    inject_touches(drag);
}

impl ViewTest for TableDragScrolling {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        // The mouse mode: the drag must not move the content.
        UIManager::set_drag_scrolling(false);
        drag_up();
        from_main(move || {
            assert!(
                view.table.content_offset().abs() < 0.001,
                "a drag scrolled with drag scrolling off"
            );
        });
        SELECTED.lock().clear();

        // The wheel still scrolls: 900 points is ten rows, so the top
        // edge tap lands on row 10.
        inject_scroll(-900);
        from_main(move || {
            assert!((view.table.content_offset() + 900.0).abs() < 0.001);
        });
        inject_touches(
            "
                200  5  b
                200  5  e
            ",
        );
        assert_eq!(SELECTED.lock().deref(), "|10|");
        SELECTED.lock().clear();

        // The finger mode: the same drag must scroll further down.
        UIManager::set_drag_scrolling(true);
        drag_up();
        from_main(move || {
            let offset = view.table.content_offset();
            assert!(
                offset < -1200.0,
                "a drag did not scroll with drag scrolling on, offset {offset}"
            );
        });
        SELECTED.lock().clear();

        Ok(())
    }
}
