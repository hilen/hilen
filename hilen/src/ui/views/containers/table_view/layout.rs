use std::ops::DerefMut;

use crate::{
    deps::refs::{Own, weak_from_ref},
    gm::LossyConvert,
    ui::{TableView, TouchStack, UIManager, ViewData, ViewFrame, ViewSubviews, WeakView},
};

pub(super) enum LayoutMode {
    Scroll,
    Resize,
    Full,
}

/// The exact inverse of `bump_z_position`, so a cell raised while its
/// row was pinned returns to the depth the bump found it at.
fn lower_z(view: WeakView, z: f32) {
    view.__base_view().z_position += z;
    for sub in view.subviews() {
        lower_z(sub.weak(), z + UIManager::subview_z_offset());
    }
}

/// How far a pinned cell rises in front of its siblings: clears several
/// nesting levels of the other cells' subtrees while staying far behind
/// the menu and modal layers.
fn sticky_raise() -> f32 {
    UIManager::subview_z_offset() * 5.0
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

        // Sticky rows on screen this frame, in content coordinates.
        let pinned = self.compute_pinned(number_of_cells, columns, header, offset);
        self.pinned = pinned.iter().map(|&(index, y, height)| (index, y + offset, height)).collect();

        let first_visible_row = rows.row_at(top);

        // `bottom` is exclusive. A row that starts exactly there is not
        // on screen, and `row_at` would still name it.
        let last_visible_row = rows.row_at(bottom);
        let mut last_row = if last_visible_row > first_visible_row && rows.top(last_visible_row) >= bottom {
            last_visible_row
        } else {
            last_visible_row + 1
        };

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
            // A full reload recycles pinned cells too, so their content is
            // rebuilt through `setup_cell` like every other row's.
            let keep_pinned = pinned.iter().any(|p| p.0 == idx);
            if !matches!(mode, LayoutMode::Full) && (keep_pinned || (idx >= first_index && idx < last_index))
            {
                shown.push(idx);
            } else {
                to_recycle.push(view.weak());
            }
        }

        let recycled: Vec<WeakView> = to_recycle
            .into_iter()
            .map(|mut cell| {
                self.lower_recycled(cell);
                cell.set_hidden(true);
                cell.as_cell().cell_removed();
                cell
            })
            .collect();
        self.registry.load_old_cells(recycled);

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

            if pinned.iter().any(|p| p.0 == i) {
                continue;
            }

            let mut cell = self.data.setup_cell(i, &mut weak_table.registry);
            let cell = cell.deref_mut();

            cell.set_tag(i);
            cell.set_frame(cell_frame(i));

            cell.as_cell().cell_added();
        }

        self.layout_pinned(&pinned, columns, cell_width, spacing);
    }

    /// The sticky rows that are on screen at the current offset, each
    /// with its pinned y in content coordinates and its height. A sticky
    /// row rides at its natural position until the viewport top passes
    /// it, pins there while its section scrolls, and the next sticky row
    /// pushes it away.
    fn compute_pinned(
        &self,
        number_of_cells: usize,
        columns: usize,
        header: f32,
        offset: f32,
    ) -> Vec<(usize, f32, f32)> {
        if self.sticky_rows.is_empty() {
            return Vec::new();
        }

        let rows = self.rows(number_of_cells);
        let viewport_top = -offset;
        let viewport_bottom = viewport_top + self.height();

        let mut pinned: Vec<(usize, f32, f32)> = self
            .sticky_rows
            .iter()
            .map(|&index| {
                let row = index / columns;
                let natural = rows.top(row) + header;
                (index, natural.max(viewport_top), rows.height(row))
            })
            .collect();

        for i in (0..pinned.len().saturating_sub(1)).rev() {
            let limit = pinned[i + 1].1 - pinned[i].2;
            if pinned[i].1 > limit {
                pinned[i].1 = limit;
            }
        }

        pinned.retain(|&(_, y, height)| y + height > viewport_top && y < viewport_bottom);
        pinned
    }

    /// Places the cells of the pinned rows, creating the ones whose row
    /// scrolled out of the recycled range, and keeps them drawn in front
    /// of the other cells while they are pinned.
    fn layout_pinned(&mut self, pinned: &[(usize, f32, f32)], columns: usize, cell_width: f32, spacing: f32) {
        self.raised.retain(|(_, view)| view.is_ok());

        if pinned.is_empty() && self.raised.is_empty() {
            return;
        }

        let mut weak_table = weak_from_ref(self);

        for &(index, y, height) in pinned {
            let existing = self
                .scroll
                .content
                .subviews()
                .iter()
                .find(|view| {
                    !view.is_hidden()
                        && view.tag() == index
                        && !self.header_views.iter().any(|h| h.raw() == view.weak().raw())
                })
                .map(Own::weak);

            let cell = if let Some(cell) = existing {
                cell
            } else {
                let mut cell = self.data.setup_cell(index, &mut weak_table.registry);
                let cell_ref = cell.deref_mut();
                cell_ref.set_tag(index);
                cell_ref.as_cell().cell_added();
                cell_ref.weak_view()
            };

            let x: f32 = (index % columns).lossy_convert() * (cell_width + spacing);
            cell.set_frame((x, y, cell_width, height));

            if !self.raised.iter().any(|(_, raised)| raised.raw() == cell.raw()) {
                cell.bump_z_position(sticky_raise());
                TouchStack::raise_subtree(cell);
                self.raised.push((index, cell));
            }
        }

        let mut lowered = Vec::new();
        self.raised.retain(|&(index, view)| {
            if pinned.iter().any(|p| p.0 == index) {
                true
            } else {
                lowered.push(view);
                false
            }
        });
        for view in lowered {
            lower_z(view, sticky_raise());
        }
    }

    /// A cell leaving for the registry drops its pinned raise first, so
    /// it comes back at a normal depth for whatever row reuses it.
    fn lower_recycled(&mut self, cell: WeakView) {
        let mut was_raised = false;
        self.raised.retain(|&(_, view)| {
            if view.raw() == cell.raw() {
                was_raised = true;
                false
            } else {
                true
            }
        });
        if was_raised {
            lower_z(cell, sticky_raise());
        }
    }
}
