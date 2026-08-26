use std::ops::DerefMut;

use crate::{
    deps::refs::weak_from_ref,
    gm::LossyConvert,
    ui::{TableView, ViewData, ViewFrame, ViewSubviews},
};

pub(super) enum LayoutMode {
    Scroll,
    Resize,
    Full,
}

impl TableView {
    pub(super) fn layout_fixed_cells(&mut self, number_of_cells: usize, columns: usize, mode: LayoutMode) {
        let spacing = self.cell_spacing;
        let width = self.width();
        let cell_width = (width - spacing * (columns - 1).lossy_convert()) / columns.lossy_convert();
        let header = self.header_height;

        // The offsets live in the table and the layout below mutates the
        // table, so the geometry reads them through a second pointer.
        let geometry_table = weak_from_ref(self);
        let rows = geometry_table.rows(number_of_cells);
        let mut weak_table = weak_from_ref(self);
        let total_height = rows.total() + header;

        self.scroll.set_content_height(total_height);
        self.scroll.set_content_width(width);

        let offset = self.scroll.get_scroll_content_offset();
        let top = -offset - header;
        let bottom = top + self.height();

        let first_visible_row = rows.row_at(top);
        let mut last_row = rows.row_at(bottom) + 1;

        // Two rows of slack past the viewport, so a small scroll shows
        // a row that is already set up.
        last_row = (last_row + 2).min(rows.count());

        let first_index = first_visible_row * columns;
        let last_index = (last_row * columns).min(number_of_cells);

        let cell_frame = |i: usize| -> (f32, f32, f32, f32) {
            let row = i / columns;
            let x: f32 = (i % columns).lossy_convert() * (cell_width + spacing);
            (x, rows.top(row) + header, cell_width, rows.height(row))
        };

        let mut to_recycle = Vec::new();
        let mut shown = Vec::new();

        for view in self.scroll.content.subviews() {
            if view.is_hidden() {
                continue;
            }

            if self.header_views.iter().any(|h| h.raw() == view.weak().raw()) {
                continue;
            }

            let idx = view.tag();
            if !matches!(mode, LayoutMode::Full) && idx >= first_index && idx < last_index {
                shown.push(idx);
            } else {
                to_recycle.push(view.weak());
            }
        }

        self.registry.load_old_cells(
            to_recycle
                .into_iter()
                .map(|mut cell| {
                    cell.set_hidden(true);
                    cell.as_cell().cell_removed();
                    cell
                })
                .collect(),
        );

        if matches!(mode, LayoutMode::Resize) {
            for view in weak_table.scroll.content.subviews() {
                if view.is_hidden() {
                    continue;
                }
                if weak_table.header_views.iter().any(|h| h.raw() == view.weak().raw()) {
                    continue;
                }
                view.set_frame(cell_frame(view.tag()));
            }
        }

        for i in first_index..last_index {
            if shown.contains(&i) {
                continue;
            }

            let mut cell = self.data.setup_cell(i, &mut weak_table.registry);
            let cell = cell.deref_mut();

            cell.set_tag(i);
            cell.set_frame(cell_frame(i));

            cell.as_cell().cell_added();
        }
    }
}
