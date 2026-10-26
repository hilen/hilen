//! Moving a row of a table to a new place by a drag, see
//! `TableView::set_row_moves` and docs/table.md.

use std::{
    mem::take,
    sync::atomic::{AtomicU64, Ordering},
};

use plat::Platform;

use super::layout::{LayoutMode, lower_z, sticky_raise};
use crate::{
    deps::{
        hreads::after,
        refs::{Weak, main_lock::MainLock, weak_from_ref},
    },
    gm::flat::Point,
    ui::{
        LongPress, Shadow, TableView, Touch, TouchStack, UIManager, View, ViewData, ViewFrame, WeakView,
        input::TouchEvent,
    },
    window::MouseButton,
};

/// How far a finger moves before it is a drag and no hold any more.
const SLOP: f32 = 10.0;
/// How long a finger holds a row still before the row lifts.
pub(crate) const HOLD: f32 = 0.4;
/// How close to the top or the bottom edge a held row scrolls the table.
const EDGE: f32 = 48.0;
/// The most points the table scrolls per step at the very edge.
const EDGE_SPEED: f32 = 14.0;
const STEP: f32 = 1.0 / 60.0;

/// The row that is held up right now.
pub(super) struct RowMove {
    from:   usize,
    to:     usize,
    /// The top of the held row in the content.
    y:      f32,
    /// Where the finger holds the row, from its top.
    grab:   f32,
    cell:   WeakView,
    shadow: Option<Shadow>,
}

#[derive(Default)]
enum Phase {
    #[default]
    Idle,
    /// A finger came down on a row. It lifts the row after a hold, or
    /// with a mouse also once it moves up or down.
    Watching {
        id:         usize,
        start:      Point,
        table:      Weak<TableView>,
        index:      usize,
        generation: u64,
    },
    /// The row follows the finger. `finger` is in screen points.
    Dragging {
        id:         usize,
        table:      Weak<TableView>,
        finger:     Point,
        generation: u64,
    },
}

static TABLES: MainLock<Vec<Weak<TableView>>> = MainLock::new();
static PHASE: MainLock<Phase> = MainLock::new();
static GENERATION: AtomicU64 = AtomicU64::new(0);

/// The gesture that lifts a row of a table with row moves on. `Input`
/// offers it every touch before the side swipe and the scrolls.
pub(crate) struct RowDrag;

impl RowDrag {
    fn register(table: Weak<TableView>, on: bool) {
        let tables = TABLES.get_mut();
        tables.retain(|t| t.is_ok() && t.raw() != table.raw());
        if on {
            tables.push(table);
        }
    }

    /// A new test, a touch another gesture took, or a screen that went
    /// away. A held row goes back with no move.
    pub(crate) fn reset() {
        if let Phase::Dragging { mut table, .. } = take(PHASE.get_mut())
            && table.is_ok()
        {
            table.end_row_move(false);
        }
    }

    /// Escape puts a held row back. True when a row was held.
    pub(crate) fn escape() -> bool {
        if !matches!(*PHASE.get_mut(), Phase::Dragging { .. }) {
            return false;
        }
        log::debug!("row move cancelled by Escape");
        Self::reset();
        true
    }

    /// True when the touch belongs to a held row and no view may see it.
    /// `touch` is in screen points.
    pub(crate) fn swallow(touch: &Touch) -> bool {
        if touch.is_began() && !matches!(*PHASE.get_mut(), Phase::Dragging { .. }) {
            *PHASE.get_mut() = Self::watch_at(touch);
            return false;
        }
        match *PHASE.get_mut() {
            Phase::Idle => false,
            Phase::Watching {
                id,
                start,
                table,
                index,
                generation,
            } => {
                if touch.id != id {
                    return false;
                }
                let moved = touch.position - start;
                match touch.event {
                    // A mouse has no scroll to keep, so a drag up or down
                    // lifts the row at once.
                    TouchEvent::Moved
                        if !UIManager::drag_scrolling()
                            && moved.y.abs() >= SLOP
                            && moved.y.abs() > moved.x.abs() =>
                    {
                        Self::lift(id, table, index, start, generation);
                        Self::drag(touch);
                        true
                    }
                    TouchEvent::Moved if moved.length() < SLOP => false,
                    _ => {
                        *PHASE.get_mut() = Phase::Idle;
                        false
                    }
                }
            }
            Phase::Dragging { id, .. } => {
                // A second finger reaches no view while the first holds a row.
                if touch.id == id {
                    Self::drag(touch);
                }
                true
            }
        }
    }

    /// The row under a finger that came down, on the front table with row
    /// moves on.
    fn watch_at(touch: &Touch) -> Phase {
        if touch.button != MouseButton::Left
            || Platform::TVOS
            || UIManager::touch_disabled()
            || TouchStack::overlay_count() > 0
        {
            return Phase::Idle;
        }
        let tables = TABLES.get_mut();
        tables.retain(Weak::is_ok);
        let Some(table) = tables
            .iter()
            .rev()
            .find(|table| !table.is_hidden_in_tree() && table.contains_visible(touch.position))
            .copied()
        else {
            return Phase::Idle;
        };
        let local = table.__base_view().local_point(touch.position);
        let Some(index) = table.index_at(local) else {
            return Phase::Idle;
        };
        if table.sticky_rows.contains(&index) || table.columns != 1 {
            return Phase::Idle;
        }
        let generation = GENERATION.fetch_add(1, Ordering::Relaxed) + 1;
        let start = touch.position;
        let id = touch.id;
        after(HOLD, move || {
            if let Phase::Watching { generation: now, .. } = *PHASE.get_mut()
                && now == generation
            {
                Self::lift(id, table, index, start, generation);
            }
        });
        Phase::Watching {
            id,
            start,
            table,
            index,
            generation,
        }
    }

    fn lift(id: usize, mut table: Weak<TableView>, index: usize, finger: Point, generation: u64) {
        if !table.is_ok() || !table.begin_row_move(index, finger) {
            *PHASE.get_mut() = Phase::Idle;
            return;
        }
        // The views and the scroll that caught the finger let it go, so
        // the release is no tap, no hold and no scroll.
        TouchStack::cancel_touch(id);
        TouchStack::release_scrolls(id);
        LongPress::cancel(id);
        log::debug!("row {index} lifted");
        *PHASE.get_mut() = Phase::Dragging {
            id,
            table,
            finger,
            generation,
        };
    }

    fn drag(touch: &Touch) {
        let Phase::Dragging {
            table,
            finger,
            generation,
            ..
        } = PHASE.get_mut()
        else {
            return;
        };
        if touch.is_moved() {
            *finger = touch.position;
        }
        let (mut table, finger, generation) = (*table, *finger, *generation);
        if !table.is_ok() {
            *PHASE.get_mut() = Phase::Idle;
            return;
        }
        if touch.is_ended() {
            *PHASE.get_mut() = Phase::Idle;
            table.end_row_move(true);
            return;
        }
        let edge = table.move_row_to(finger);
        if edge != 0.0 {
            Self::scroll_soon(generation);
        }
    }

    /// While the finger rests at an edge the table goes on scrolling.
    fn scroll_soon(generation: u64) {
        after(STEP, move || {
            let Phase::Dragging {
                mut table,
                finger,
                generation: now,
                ..
            } = *PHASE.get_mut()
            else {
                return;
            };
            if now != generation || !table.is_ok() {
                return;
            }
            let step = table.edge_step(finger);
            if step == 0.0 {
                return;
            }
            let offset = table.content_offset() + step;
            table.set_content_offset(offset);
            table.move_row_to(finger);
            Self::scroll_soon(generation);
        });
    }
}

impl TableView {
    /// A row can be dragged to a new place. A press and a move up or down
    /// lifts it with a mouse, a hold of 0.4 seconds with a finger, where a
    /// move before that still scrolls. The held row follows the pointer in
    /// front of the others with a shadow, the rows between its old and its
    /// new place move aside, and a pointer near the top or the bottom edge
    /// scrolls the table. The release calls `TableData::move_row` and
    /// reloads the table. Escape, and a reload while the row is held, put
    /// it back with no call. Sticky rows do not move. A table of 1 column
    /// only. Off by default.
    pub fn set_row_moves(&mut self, on: bool) -> &mut Self {
        RowDrag::register(weak_from_ref(self), on);
        self
    }

    fn begin_row_move(&mut self, index: usize, finger: Point) -> bool {
        let Some(cell) = self
            .visible_cells()
            .into_iter()
            .find_map(|(i, cell)| (i == index).then_some(cell))
        else {
            return false;
        };
        let content_y = self.__base_view().local_point(finger).y - self.content_offset();
        let shadow = cell.shadow();
        cell.bump_z_position(sticky_raise());
        cell.set_shadow(Shadow::default());
        self.row_move = Some(RowMove {
            from: index,
            to: index,
            y: cell.y(),
            grab: content_y - cell.y(),
            cell,
            shadow,
        });
        true
    }

    /// Puts the held row under the finger and the other rows aside. Gives
    /// the step the table scrolls by at an edge, 0 away from them.
    fn move_row_to(&mut self, finger: Point) -> f32 {
        let Some(number_of_cells) = self.laid_cells else {
            return 0.0;
        };
        let header = self.header_height;
        let content_y = self.__base_view().local_point(finger).y - self.content_offset();
        let rows = self.rows(number_of_cells);
        let Some(held) = &self.row_move else {
            return 0.0;
        };
        let from = held.from;
        let height = rows.height(from);
        let y = (content_y - held.grab).clamp(header, header + rows.total() - height);
        let center = y + height / 2.0 - header;
        // A held row at the very end has its center on the center of the
        // last row, the tie goes to the side the row moved to.
        let to = (0..rows.count())
            .filter(|&row| {
                let middle = rows.top(row) + rows.height(row) / 2.0;
                (row < from && middle < center) || (row > from && middle <= center)
            })
            .count();
        if let Some(held) = &mut self.row_move {
            held.y = y;
            if held.to != to {
                log::debug!("held row {from} is over place {to}");
            }
            held.to = to;
        }
        self.apply_row_move();
        self.edge_step(finger)
    }

    fn edge_step(&self, finger: Point) -> f32 {
        let y = self.__base_view().local_point(finger).y;
        let height = self.height();
        if y < EDGE {
            EDGE_SPEED * (1.0 - y.max(0.0) / EDGE)
        } else if y > height - EDGE {
            -EDGE_SPEED * (1.0 - (height - y).max(0.0) / EDGE)
        } else {
            0.0
        }
    }

    /// Gives the cells on screen their places while a row is held. Runs
    /// after every layout too, a row that scrolls into view comes in at
    /// its natural place.
    pub(super) fn apply_row_move(&mut self) {
        let Some(held) = &self.row_move else {
            return;
        };
        let Some(number_of_cells) = self.laid_cells else {
            return;
        };
        let (from, to, y) = (held.from, held.to, held.y);
        let rows = self.rows(number_of_cells);
        let pitch = rows.height(from) + self.cell_spacing;
        let header = self.header_height;
        for (index, cell) in self.visible_cells() {
            let top = if index == from {
                y
            } else if from < to && index > from && index <= to {
                rows.top(index) + header - pitch
            } else if to < from && index >= to && index < from {
                rows.top(index) + header + pitch
            } else {
                rows.top(index) + header
            };
            let frame = *cell.frame();
            cell.set_frame((frame.origin.x, top, frame.size.width, frame.size.height));
        }
    }

    /// The held cell stays when its row scrolls out of view.
    pub(super) fn holds_row(&self, index: usize) -> bool {
        self.row_move.as_ref().is_some_and(|held| held.from == index)
    }

    /// Drops a held row before a reload, the rows under it may be others.
    /// The finger stays with the gesture until it is up, so it moves no
    /// row and reaches no view.
    pub(super) fn drop_row_move(&mut self) {
        if let Some(held) = self.row_move.take() {
            log::debug!("a reload put the held row {} back", held.from);
            put_down(&held);
        }
    }

    fn end_row_move(&mut self, commit: bool) {
        let Some(held) = self.row_move.take() else {
            return;
        };
        put_down(&held);
        if commit && held.from != held.to {
            log::debug!("row {} moved to {}", held.from, held.to);
            self.data.move_row(held.from, held.to);
            self.reload_data();
        } else {
            self.layout_cells(LayoutMode::Resize);
        }
    }
}

/// The held cell goes back to the depth and the shadow it had.
fn put_down(held: &RowMove) {
    if held.cell.is_ok() {
        lower_z(held.cell, sticky_raise());
        held.cell.set_shadow(held.shadow);
    }
}
