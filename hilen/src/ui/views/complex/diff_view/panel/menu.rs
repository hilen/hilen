//! The discard of a row from a menu, for a long press on a touch screen,
//! which has no pointer to show the discard buttons, and a right click.

use super::{DiffPanel, DiffRow};
use crate::{
    refs::Weak,
    ui::{ContextMenu, MenuItem, Point},
};

/// What a row can discard, the same as its discard button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Discard {
    Hunk(usize),
    Line { hunk: usize, line: usize },
}

impl DiffPanel {
    /// The discard of the row and half under a panel position, none for a
    /// line that did not change or a panel that cannot discard.
    pub(super) fn discard_at(self: Weak<Self>, position: Point) -> Option<Discard> {
        if !self.can_discard {
            return None;
        }
        let (index, _, right) = self.sel_pos(position, None)?;
        match self.rows.get(index)? {
            DiffRow::Hunk { hunk_index, .. } => Some(Discard::Hunk(*hunk_index)),
            DiffRow::Line {
                kind,
                hunk_index,
                line_index,
                ..
            } => (*kind != ' ').then_some(Discard::Line {
                hunk: *hunk_index,
                line: *line_index,
            }),
            DiffRow::Split {
                left,
                right: right_half,
                hunk_index,
            } => {
                let half = if right { right_half } else { left }.as_ref()?;
                (half.kind != ' ').then_some(Discard::Line {
                    hunk: *hunk_index,
                    line: half.line_index,
                })
            }
            DiffRow::Note(_) => None,
        }
    }

    pub(super) fn open_discard_menu(self: Weak<Self>, position: Point) {
        let item = match self.discard_at(position) {
            Some(Discard::Hunk(hunk)) => MenuItem::new("Discard hunk", move || self.discard_hunk(hunk)),
            Some(Discard::Line { hunk, line }) => {
                MenuItem::new("Discard line", move || self.discard_line(hunk, line))
            }
            None => return,
        };
        ContextMenu::show_at_cursor(vec![item.danger()]);
    }
}
