//! A change of 1 cell and new cells at the end, with no full reload.

use std::ops::Deref;

use super::layout::LayoutMode;
use crate::ui::{TableView, WeakView};

impl TableView {
    /// The cell `index` changed, its content, its height or both. The
    /// table asks `cell_height` for the row of that cell only, moves the
    /// rows under it, and calls `setup_cell` for that cell when it is on
    /// screen. No other row is asked and no other cell is set up, so a
    /// row that grows many times a second, like the last line of a chat,
    /// costs the same in a table of 10 rows and of 10 000.
    ///
    /// The height is read only with `set_variable_heights(true)`. The
    /// scroll offset stays, call `scroll_to_bottom` after it to follow a
    /// last row that grows. The number of cells must be the one of the
    /// last reload, a table whose count changed reloads everything here.
    pub fn reload_cell(&mut self, index: usize) {
        if self.data.is_null() {
            return;
        }

        let number_of_cells = self.data.number_of_cells();
        if self.laid_cells != Some(number_of_cells) || index >= number_of_cells {
            self.reload_data();
            return;
        }

        if self.variable_heights {
            let row = index / self.columns;
            let height = self.data.cell_height(row * self.columns);
            let end = self.row_offsets[row] + height + self.cell_spacing;
            let moved = end - self.row_offsets[row + 1];
            if moved != 0.0 {
                for offset in &mut self.row_offsets[row + 1..] {
                    *offset += moved;
                }
            }
        }

        // A cell on screen goes to the registry and comes back from it
        // through `setup_cell` in the layout below.
        let cell: Vec<WeakView> = self
            .visible_cells()
            .into_iter()
            .filter(|(shown, _)| *shown == index)
            .map(|(_, cell)| cell)
            .collect();
        self.recycle(cell);

        // The mode that gives every cell on screen its frame again, the
        // rows under the changed one moved.
        self.layout_cells(LayoutMode::Resize);
    }

    /// The data got more cells at its end. The table asks `cell_height`
    /// for the new rows only and sets up the new cells that are on
    /// screen. The rows before them are not asked and their cells stay
    /// as they are. Call `scroll_to_bottom` after it to show the new
    /// rows. A table whose count went down, or that has no layout yet,
    /// reloads everything here.
    pub fn load_new_cells(&mut self) {
        if self.data.is_null() {
            return;
        }

        let number_of_cells = self.data.number_of_cells();
        let Some(before) = self.laid_cells.filter(|before| *before <= number_of_cells) else {
            self.reload_data();
            return;
        };
        if before == number_of_cells {
            return;
        }

        if self.variable_heights {
            let columns = self.columns;
            let spacing = self.cell_spacing;
            let data = self.data;
            let mut end = self.row_offsets.last().copied().unwrap_or_default();
            for row in self.row_offsets.len() - 1..self.row_count(number_of_cells) {
                end += data.deref().cell_height(row * columns) + spacing;
                self.row_offsets.push(end);
            }
        }

        if self.sticky_enabled {
            let data = self.data;
            self.sticky_rows
                .extend((before..number_of_cells).filter(|index| data.deref().is_sticky(*index)));
        }

        self.laid_cells = Some(number_of_cells);
        self.layout_cells(LayoutMode::Scroll);
    }
}
