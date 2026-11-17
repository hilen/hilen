//! A table whose rows have levels and fold, like the epics, tasks and
//! subtasks of a planner or the folders of a file tree. The app gives every
//! row in the order of the tree, a row and then the rows under it, each with
//! its level. A row whose next row is deeper has rows under it and gets an
//! arrow, a tap on the arrow hides and shows them.

mod fold_arrow;
#[cfg(feature = "ui-tests")]
mod tree_table_folds;

use std::collections::HashSet;

use log::debug;

pub use self::fold_arrow::FoldArrow;
use crate::{
    self as hilen,
    deps::{
        refs::{Weak, weak_from_ref},
        vents::Event,
    },
    gm::LossyConvert,
    ui::{CellRegistry, Setup, TableData, TableView, UIColor, View, ViewData, ViewSubviews, view},
};

/// The width the fold arrow takes at the left of a row, over the whole
/// height of the row, so a finger finds it.
pub const FOLD_ARROW_SIZE: f32 = 28.0;

/// The rows of a `TreeTable`, every row whether it is folded away or not.
pub trait TreeData {
    /// Every row, in the order of the tree.
    fn number_of_rows(&self) -> usize;

    /// How deep the row is, 0 at the left edge. The rows right after a row
    /// that are deeper than it are the rows under it.
    fn level(&self, row: usize) -> usize;

    /// A key that stays with the row over a reload, like the id of the item
    /// it shows. The tree keeps which rows are folded by it.
    fn row_key(&self, row: usize) -> u64;

    fn row_height(&self, row: usize) -> f32;

    /// The cell of a row, from the registry like a cell of a `TableView`.
    /// Its content starts at `TreeTable::content_left` of the level, the
    /// fold arrow stands left of it.
    fn setup_row(&mut self, row: usize, registry: &mut CellRegistry) -> Weak<dyn View>;

    fn row_selected(&mut self, row: usize);
}

/// A `TableView` that shows only the rows no folded row hides, and puts a
/// fold arrow into each cell of a row with rows under it.
#[view]
pub struct TreeTable {
    /// A row folded, true, or opened, false, by a tap on its arrow, with the
    /// key of the row.
    pub folded: Event<(u64, bool)>,

    data:   Weak<dyn TreeData>,
    /// The keys of the folded rows.
    closed: HashSet<u64>,
    /// The row of the data each line of the table shows.
    shown:  Vec<usize>,
    indent: f32,
    tint:   Option<UIColor>,

    #[init]
    table: TableView,
}

impl Setup for TreeTable {
    fn setup(mut self: Weak<Self>) {
        self.indent = 20.0;
        self.table.place().back();
        self.table.set_variable_heights(true);
        self.table.set_data_source(self);
    }
}

impl TreeTable {
    pub fn set_data_source(mut self: Weak<Self>, data: Weak<dyn TreeData>) -> Weak<Self> {
        self.data = data;
        self
    }

    pub fn register_cell<T: View + Default + 'static>(self: Weak<Self>) -> Weak<Self> {
        self.table.register_cell::<T>();
        self
    }

    /// The table under the tree, for its spacing, colors and scroll calls.
    /// Its cells are the shown rows, `shown_row` gives the row of the data.
    pub fn table(self: Weak<Self>) -> Weak<TableView> {
        self.table
    }

    /// The rows of the data changed, or which are folded.
    pub fn reload_data(mut self: Weak<Self>) {
        self.find_shown();
        self.table.reload_data();
    }

    /// How far a level moves its row right, 20 by default.
    pub fn set_indent(mut self: Weak<Self>, indent: f32) {
        self.indent = indent;
        self.table.reload_data();
    }

    /// The color of the arrows, the text color of the theme by default.
    pub fn set_arrow_tint(mut self: Weak<Self>, tint: impl Into<UIColor>) {
        self.tint = Some(tint.into());
        self.table.reload_data();
    }

    /// Where the content of a row of `level` starts, right of its arrow.
    pub fn content_left(self: Weak<Self>, level: usize) -> f32 {
        self.arrow_left(level) + FOLD_ARROW_SIZE
    }

    pub fn is_folded(self: Weak<Self>, key: u64) -> bool {
        self.closed.contains(&key)
    }

    /// Folds or opens a row from code, with no `folded` event.
    pub fn set_folded(mut self: Weak<Self>, key: u64, folded: bool) {
        if folded {
            self.closed.insert(key);
        } else {
            self.closed.remove(&key);
        }
        self.reload_data();
    }

    /// The row of the data that line `index` of the table shows.
    pub fn shown_row(self: Weak<Self>, index: usize) -> Option<usize> {
        self.shown.get(index).copied()
    }

    /// How many rows are in sight, the folded ones left out.
    pub fn shown_count(self: Weak<Self>) -> usize {
        self.shown.len()
    }

    fn arrow_left(self: Weak<Self>, level: usize) -> f32 {
        let level: f32 = level.lossy_convert();
        level * self.indent
    }

    fn has_rows_under(self: Weak<Self>, row: usize) -> bool {
        row + 1 < self.data.number_of_rows() && self.data.level(row + 1) > self.data.level(row)
    }

    fn find_shown(mut self: Weak<Self>) {
        let mut shown = Vec::new();
        // The level of a folded row while its rows are skipped.
        let mut folded_at: Option<usize> = None;
        for row in 0..self.data.number_of_rows() {
            let level = self.data.level(row);
            if let Some(folded) = folded_at {
                if level > folded {
                    continue;
                }
                folded_at = None;
            }
            shown.push(row);
            if self.closed.contains(&self.data.row_key(row)) && self.has_rows_under(row) {
                folded_at = Some(level);
            }
        }
        debug!(
            "the tree shows {} of {} rows",
            shown.len(),
            self.data.number_of_rows()
        );
        self.shown = shown;
    }

    fn toggle(mut self: Weak<Self>, key: u64) {
        let fold = !self.closed.contains(&key);
        if fold {
            self.closed.insert(key);
        } else {
            self.closed.remove(&key);
        }
        debug!("tree row {key} {}", if fold { "folds" } else { "opens" });
        self.reload_data();
        self.folded.trigger((key, fold));
    }

    /// The arrow of a cell: the one it got when it showed another row, or
    /// a new one. It is the last subview, so it is in front of the content.
    fn place_arrow(self: Weak<Self>, cell: Weak<dyn View>, row: usize) {
        let found = cell.subviews().iter().find_map(|view| view.downcast_view::<FoldArrow>());
        let arrow = found.unwrap_or_else(|| {
            let arrow = cell.add_view::<FoldArrow>();
            arrow.tapped.val(move |key| self.toggle(key));
            arrow
        });
        let key = self.data.row_key(row);
        arrow.show(
            key,
            self.has_rows_under(row),
            !self.closed.contains(&key),
            self.tint,
        );
        arrow
            .place()
            .clear()
            .l(self.arrow_left(self.data.level(row)))
            .tb(0)
            .w(FOLD_ARROW_SIZE);
    }
}

impl TableData for TreeTable {
    fn cell_height(&self, index: usize) -> f32 {
        self.data.row_height(self.shown[index])
    }

    fn number_of_cells(&self) -> usize {
        self.shown.len()
    }

    fn cell_selected(&mut self, index: usize) {
        let row = self.shown[index];
        self.data.row_selected(row);
    }

    fn setup_cell(&mut self, index: usize, registry: &mut CellRegistry) -> Weak<dyn View> {
        let row = self.shown[index];
        let cell = self.data.setup_row(row, registry);
        weak_from_ref(self).place_arrow(cell, row);
        cell
    }
}
