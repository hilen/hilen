//! The selectable side of a table: the flag, and the text of a row that
//! is not on screen.

use crate::{
    deps::refs::weak_from_ref,
    gm::flat::Point,
    ui::{
        TableView, View, ViewData,
        text_selection::{Lead, texts},
    },
};

impl TableView {
    /// The text of the cells can be selected with the mouse and copied,
    /// like the text of a text editor. Off by default. One selection goes
    /// over many cells, also cells of different kinds. Inside a cell the
    /// selectable text is every `Label` and `MarkdownView` with
    /// `set_selectable(true)`.
    ///
    /// A drag selects and scrolls the table when it reaches the top or
    /// the bottom edge. A double click takes a word, a triple click a
    /// line, Shift with a click extends, Cmd or Ctrl with C copies, with
    /// A takes the text of every row, and a right click opens a menu with
    /// Copy. On a touch screen a long press selects. The copy has 1 line
    /// break between 2 views.
    ///
    /// The selection is kept as positions in the data, so it stays when
    /// its rows scroll out and come back. A copy and Select All call
    /// `setup_cell` for the selected rows that are not on screen, to read
    /// their text.
    pub fn set_text_selectable(&mut self, selectable: bool) -> &mut Self {
        self.text_selectable = selectable;
        self
    }

    pub fn is_text_selectable(&self) -> bool {
        self.text_selectable
    }

    /// How many rows a selection can go over.
    pub(crate) fn selectable_rows(&self) -> usize {
        if self.data.is_null() {
            0
        } else {
            self.data.number_of_cells()
        }
    }

    /// The selectable texts of the cell `row`. A row on screen is read
    /// from its cell. Any other row gets a cell from the registry for
    /// the read, which goes back at once.
    pub(crate) fn row_texts(&mut self, row: usize) -> Vec<(String, Lead)> {
        if let Some((_, cell)) = self.visible_cells().into_iter().find(|(index, _)| *index == row) {
            return texts(cell);
        }
        if self.data.is_null() || row >= self.data.number_of_cells() {
            return Vec::new();
        }

        let mut table = weak_from_ref(self);
        let mut cell = self.data.setup_cell(row, &mut table.registry);
        let found = texts(cell);

        cell.set_hidden(true);
        cell.as_cell().cell_removed();
        self.registry.load_old_cells(vec![cell]);

        found
    }

    /// A tap at a point on the screen, for a press a view inside a cell
    /// took for the text selection.
    pub(crate) fn tap_at(&self, point: Point) {
        weak_from_ref(self).select_at(self.__base_view().local_point(point));
    }
}
