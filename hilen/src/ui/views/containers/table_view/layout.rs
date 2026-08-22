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

fn cell_frame(
    i: usize,
    columns: usize,
    cell_width: f32,
    cell_height: f32,
    spacing: f32,
    header: f32,
) -> (f32, f32, f32, f32) {
    let x: f32 = (i % columns).lossy_convert() * (cell_width + spacing);
    let y: f32 = (i / columns).lossy_convert() * (cell_height + spacing) + header;
    (x, y, cell_width, cell_height)
}

impl TableView {
    pub(super) fn layout_fixed_cells(&mut self, number_of_cells: usize, columns: usize, mode: LayoutMode) {
        let cell_height = self.data.cell_height(0);
        let spacing = self.cell_spacing;
        let row_pitch = cell_height + spacing;
        let width = self.width();
        let cell_width = (width - spacing * (columns - 1).lossy_convert()) / columns.lossy_convert();

        let rows: f32 = (number_of_cells.lossy_convert() / columns.lossy_convert()).ceil();
        let total_height = rows * row_pitch - spacing + self.header_height;

        self.scroll.set_content_height(total_height);
        self.scroll.set_content_width(width);

        let rows_fit: usize = (self.height() / row_pitch).ceil().lossy_convert();
        let offset = self.scroll.get_scroll_content_offset();
        let first_visible_row: usize =
            ((-offset - self.header_height) / row_pitch).floor().max(0.0).lossy_convert();
        let first_index = first_visible_row * columns;

        let mut last_index = first_index + rows_fit * columns + columns * 2;
        if last_index > number_of_cells {
            last_index = number_of_cells;
        }

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

        let mut weak_table = weak_from_ref(self);

        if matches!(mode, LayoutMode::Resize) {
            for view in weak_table.scroll.content.subviews() {
                if view.is_hidden() {
                    continue;
                }
                if weak_table.header_views.iter().any(|h| h.raw() == view.weak().raw()) {
                    continue;
                }
                view.set_frame(cell_frame(
                    view.tag(),
                    columns,
                    cell_width,
                    cell_height,
                    spacing,
                    self.header_height,
                ));
            }
        }

        for i in first_index..last_index {
            if shown.contains(&i) {
                continue;
            }

            let mut cell = self.data.setup_cell(i, &mut weak_table.registry);
            let cell = cell.deref_mut();

            cell.set_tag(i);
            cell.set_frame(cell_frame(
                i,
                columns,
                cell_width,
                cell_height,
                spacing,
                self.header_height,
            ));

            cell.as_cell().cell_added();
        }
    }
}
