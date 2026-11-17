//! A change of 1 cell and new cells at the end, with no full reload.

use std::ops::Deref;

use super::layout::LayoutMode;
use crate::{
    gm::LossyConvert,
    ui::{TableView, TextSelection, ViewData, WeakView},
};

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

    /// The first `count` cells left the data. The table asks no
    /// `cell_height` and sets up no cell that stays on screen: the rows
    /// that are left keep their heights and their cells, and stay at the
    /// same place on the screen, the scroll offset moves by the height
    /// that is gone. A text selection stays on its text.
    ///
    /// The cells after them must be the ones of the last reload. A table
    /// whose count does not fit, a table with sticky rows, and a count
    /// that is not whole rows reload everything here.
    pub fn drop_first_cells(&mut self, count: usize) {
        if count == 0 || self.data.is_null() {
            return;
        }
        TextSelection::rows_dropped(self, count);

        let number_of_cells = self.data.number_of_cells();
        let fits = number_of_cells > 0
            && !self.sticky_enabled
            && count.is_multiple_of(self.columns)
            && self.laid_cells == Some(number_of_cells + count);
        if !fits {
            self.reload_data();
            return;
        }

        let rows = count / self.columns;
        let removed = if self.variable_heights {
            let removed = self.row_offsets[rows];
            self.row_offsets.drain(..rows);
            for offset in &mut self.row_offsets {
                *offset -= removed;
            }
            removed
        } else {
            let rows: f32 = rows.lossy_convert();
            rows * (self.data.cell_height(0) + self.cell_spacing)
        };

        let mut gone = Vec::new();
        for (index, mut cell) in self.visible_cells() {
            if index < count {
                gone.push(cell);
            } else {
                cell.set_tag(index - count);
            }
        }
        self.recycle(gone);

        let offset = self.scroll.get_scroll_content_offset();
        self.scroll.set_content_offset((offset + removed).min(0.0));
        self.laid_cells = Some(number_of_cells);
        self.layout_cells(LayoutMode::Resize);
    }
}
