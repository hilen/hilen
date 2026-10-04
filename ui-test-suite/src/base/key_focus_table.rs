use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{CellRegistry, Focus, Label, NamedKey, Setup, TableData, TableView, View, ViewData, ViewTest, view},
    ui_test::{check_colors, checkpoint, inject_named_key},
};

const ROWS: usize = 30;

/// A table takes the taps of its cells itself and has no views past the
/// rows it shows. The ring still walks cell by cell, the table scrolls
/// under it, and Enter selects the cell.
#[view]
struct KeyFocusTable {
    selected: Option<usize>,

    #[init]
    title: Label,
    table: TableView,
    state: Label,
}

impl KeyFocusTable {
    fn focused_row(self: Weak<Self>) -> Option<usize> {
        from_main(move || {
            let focused = Focus::focused();
            let row = focused.is_ok().then(|| focused.tag());
            self.state.set_text(format!(
                "ring on row: {}, selected: {}",
                row.map_or_else(|| "none".into(), |row| (row + 1).to_string()),
                self.selected.map_or_else(|| "none".into(), |row| (row + 1).to_string()),
            ));
            row
        })
    }
}

/// A move past the last shown row scrolls first and lands when the table
/// has made the new rows.
fn press(key: NamedKey) {
    inject_named_key(key);
    for _ in 0..5 {
        wait_for_next_frame();
    }
}

impl Setup for KeyFocusTable {
    fn setup(self: Weak<Self>) {
        self.title.set_text("down walks past the shown rows").set_text_size(20);
        self.title.place().lrt(10).h(40);

        self.table.place().lr(60).t(60).b(70);
        self.table.set_data_source(self).register_cell::<Label>();

        self.state.set_text_size(20);
        self.state.place().lrb(10).h(40);
    }
}

impl TableData for KeyFocusTable {
    fn cell_height(&self, _: usize) -> f32 {
        60.0
    }

    fn number_of_cells(&self) -> usize {
        ROWS
    }

    fn setup_cell(&mut self, index: usize, registry: &mut CellRegistry) -> Weak<dyn View> {
        let label = registry.cell::<Label>();
        label.set_text(format!("row {}", index + 1)).set_text_size(22);
        label.set_color(if index.is_multiple_of(2) {
            "#e3e8ef"
        } else {
            "#f4f6f9"
        });
        label
    }

    fn cell_selected(&mut self, index: usize) {
        self.selected = Some(index);
    }
}

impl ViewTest for KeyFocusTable {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        press(NamedKey::ArrowDown);
        ensure!(
            view.focused_row() == Some(0),
            "the first key did not show the ring on row 1"
        );
        wait_for_next_frame();
        check_colors(RING_ON_ROW_1)?;

        for step in 1..=14 {
            press(NamedKey::ArrowDown);
            let row = view.focused_row();
            ensure!(
                row == Some(step),
                "after {step} moves down the ring is on {row:?}"
            );
        }
        wait_for_next_frame();
        checkpoint("the ring is on row 15, the table scrolled to it")?;

        inject_named_key(NamedKey::Enter);
        wait_for_next_frame();
        view.focused_row();
        ensure!(
            from_main(move || view.selected) == Some(14),
            "Enter did not select row 15"
        );
        checkpoint("Enter selected row 15")?;

        for step in (10..14).rev() {
            press(NamedKey::ArrowUp);
            let row = view.focused_row();
            ensure!(
                row == Some(step),
                "moving up the ring is on {row:?}, expected {step}"
            );
        }
        wait_for_next_frame();
        checkpoint("up walks back, the ring is on row 11")?;

        Ok(())
    }
}

const RING_ON_ROW_1: &str = "";
