//! Which texts a scope has and which of them is under a point.

use super::{
    position::{Granularity, Lead, Position, Scope},
    words::{line_range, word_range},
};
use crate::{
    deps::refs::Weak,
    gm::flat::{Point, Rect},
    ui::{Label, MarkdownView, TableView, View, ViewData, ViewFrame, ViewSubviews, WeakView},
};

/// The selectable labels under `view` in reading order, the hidden ones
/// left out. The index of a label here is the `piece` of a `Position`.
pub(crate) fn pieces(view: WeakView) -> Vec<Weak<Label>> {
    let mut found = Vec::new();
    collect_pieces(view, &mut found);
    found
}

fn collect_pieces(view: WeakView, found: &mut Vec<Weak<Label>>) {
    if view.is_null() || view.is_hidden() {
        return;
    }
    if let Some(label) = view.downcast_view::<Label>() {
        if label.is_selectable() {
            found.push(label);
        }
        return;
    }
    if let Some(markdown) = view.downcast_view::<MarkdownView>()
        && markdown.is_selectable()
    {
        found.extend(markdown.text_labels());
        return;
    }
    for sub in view.subviews() {
        collect_pieces(sub.weak(), found);
    }
}

/// The same pieces as `pieces` gives, as their texts and what stands
/// before each in a copy. It reads no layout, so it also works on a cell
/// that was just set up and has no frame yet.
pub(crate) fn texts(view: WeakView) -> Vec<(String, Lead)> {
    let mut found = Vec::new();
    collect_texts(view, &mut found);
    found
}

fn collect_texts(view: WeakView, found: &mut Vec<(String, Lead)>) {
    if view.is_null() || view.is_hidden() {
        return;
    }
    if let Some(label) = view.downcast_view::<Label>() {
        if label.is_selectable() {
            found.push((label.text().to_string(), Lead::Line));
        }
        return;
    }
    if let Some(markdown) = view.downcast_view::<MarkdownView>()
        && markdown.is_selectable()
    {
        let mut texts = markdown.flat_texts();
        // The lead of the first text is the one between 2 views.
        if let Some(first) = texts.first_mut() {
            first.1 = Lead::Line;
        }
        found.append(&mut texts);
        return;
    }
    for sub in view.subviews() {
        collect_texts(sub.weak(), found);
    }
}

/// Whether a press on `view` starts a selection. A selectable label, a
/// selectable markdown view and a table with selectable text take their
/// touches for it. Any other view inside them, a button in a cell, keeps
/// its touch.
pub(crate) fn is_target(view: WeakView) -> bool {
    if view.is_null() {
        return false;
    }
    if let Some(label) = view.downcast_view::<Label>() {
        return label.is_selectable();
    }
    if let Some(markdown) = view.downcast_view::<MarkdownView>() {
        return markdown.is_selectable();
    }
    view.downcast_view::<TableView>()
        .is_some_and(|table| table.is_text_selectable())
}

/// The scope a target belongs to: the table around it when that table
/// has selectable text, else the markdown view around it, else itself.
pub(crate) fn scope_of(view: WeakView) -> Option<Scope> {
    let mut root: Option<WeakView> = None;
    let mut current = view;

    while current.is_ok() {
        if let Some(table) = current.downcast_view::<TableView>()
            && table.is_text_selectable()
        {
            return Some(Scope::Table(table));
        }
        let markdown = current.downcast_view::<MarkdownView>().is_some_and(|view| view.is_selectable());
        let label = current.downcast_view::<Label>().is_some_and(|view| view.is_selectable());
        if markdown || (label && root.is_none()) {
            root = Some(current);
        }
        current = *current.superview();
    }

    root.map(Scope::View)
}

/// The text under a point.
pub(crate) struct Hit {
    pub position: Position,
    /// The label of the piece, null for a row with no selectable text.
    pub label:    Weak<Label>,
}

impl Hit {
    /// What a click with this granularity takes around the hit.
    pub fn unit(&self, granularity: Granularity) -> (Position, Position) {
        let at = self.position;
        if self.label.is_null() {
            return (at, at);
        }
        let range = match granularity {
            Granularity::Char => return (at, at),
            Granularity::Word => word_range(self.label.text(), at.byte),
            Granularity::Line => line_range(self.label.text(), at.byte),
        };
        (
            Position::new(at.row, at.piece, range.start),
            Position::new(at.row, at.piece, range.end),
        )
    }
}

/// How far `point` is from `frame`, down first and sideways second, so
/// a point beside a line of text stays on that line.
fn distance(frame: &Rect, point: Point) -> (f32, f32) {
    let outside = |value: f32, min: f32, max: f32| {
        if value < min {
            min - value
        } else if value > max {
            value - max
        } else {
            0.0
        }
    };
    (
        outside(point.y, frame.y(), frame.max_y()),
        outside(point.x, frame.x(), frame.max_x()),
    )
}

/// The piece of `view` closest to `point`, a point on the screen.
fn nearest(view: WeakView, row: usize, point: Point) -> Hit {
    let found = pieces(view).into_iter().enumerate().min_by(|(_, a), (_, b)| {
        let a = distance(a.absolute_frame(), point);
        let b = distance(b.absolute_frame(), point);
        a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1))
    });

    let Some((piece, label)) = found else {
        return Hit {
            position: Position::new(row, 0, 0),
            label:    Weak::default(),
        };
    };

    let byte = label.byte_at(label.__base_view().local_point(point));
    Hit {
        position: Position::new(row, piece, byte),
        label,
    }
}

/// The text of `scope` closest to `point`, a point on the screen. None
/// for a table that shows no cell.
pub(crate) fn hit(scope: Scope, point: Point) -> Option<Hit> {
    match scope {
        Scope::View(view) => Some(nearest(view, 0, point)),
        Scope::Table(table) => {
            let (row, cell) = table.visible_cells().into_iter().min_by(|(_, a), (_, b)| {
                let a = distance(a.absolute_frame(), point);
                let b = distance(b.absolute_frame(), point);
                a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1))
            })?;
            Some(nearest(cell, row, point))
        }
    }
}
