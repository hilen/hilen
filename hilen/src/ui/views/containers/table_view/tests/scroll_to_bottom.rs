use std::ops::Deref;

use anyhow::Result;
use parking_lot::Mutex;

use crate::{
    self as hilen,
    deps::{hreads::from_main, refs::Weak},
    ui::{CellRegistry, Container, Setup, TableData, TableView, View, ViewData, ViewTest, view},
    ui_test::inject_touches,
};

static SELECTED: Mutex<String> = Mutex::new(String::new());

// Taps prove the scroll position exactly: after a jump to the bottom the
// last row must sit flush with the table bottom, so a tap near the bottom
// edge selects the last index and a tap near the top selects the first
// fully visible row.
#[view]
struct TableScrollToBottom {
    rows: usize,

    #[init]
    table: TableView,
}

impl Setup for TableScrollToBottom {
    fn setup(mut self: Weak<Self>) {
        self.rows = 100;
        self.table.place().tl(0).size(400, 600);
        self.table.set_data_source(self).register_cell::<Container>();
        self.table.reload_data();
    }
}

impl TableData for TableScrollToBottom {
    fn cell_height(&self, _: usize) -> f32 {
        90.0
    }

    fn number_of_cells(&self) -> usize {
        self.rows
    }

    fn setup_cell(&mut self, _index: usize, registry: &mut CellRegistry) -> Weak<dyn View> {
        registry.cell::<Container>()
    }

    fn cell_selected(&mut self, index: usize) {
        *SELECTED.lock() += &format!("|{index}|");
    }
}

impl ViewTest for TableScrollToBottom {
    fn perform_test(mut view: Weak<Self>) -> Result<()> {
        // Unscrolled the top row is 0.
        inject_touches(
            "
                200  5    b
                200  5    e
            ",
        );
        assert_eq!(SELECTED.lock().deref(), "|0|");
        SELECTED.lock().clear();

        from_main(move || {
            view.table.scroll_to_bottom();
        });

        // 100 rows of 90 against a 600 viewport puts the offset at -8400,
        // so the bottom edge hits row 99 and the top edge hits row 93.
        inject_touches(
            "
                200  595  b
                200  595  e
                200  5    b
                200  5    e
            ",
        );
        assert_eq!(SELECTED.lock().deref(), "|99||93|");
        SELECTED.lock().clear();

        // The log tail flow: append rows, reload, jump to the bottom again.
        from_main(move || {
            view.rows = 120;
            view.table.reload_data();
            view.table.scroll_to_bottom();
        });

        inject_touches(
            "
                200  595  b
                200  595  e
            ",
        );
        assert_eq!(SELECTED.lock().deref(), "|119|");
        SELECTED.lock().clear();

        Ok(())
    }
}
