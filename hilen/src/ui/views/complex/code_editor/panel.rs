//! Views that float next to a place of the code of a `CodeEditor`: a
//! panel under the line, like what an app knows of the name under the
//! pointer, and a note over the line, like the signature of the call whose
//! arguments are typed. They take no keys, the typing stays with the
//! editor.

use std::mem::take;

use super::{text::CodePos, view::CodeEditor};
use crate::{
    deps::refs::{Own, Weak, weak_from_ref},
    gm::flat::Rect,
    ui::{UIManager, View, ViewData, ViewFrame, ViewSubviews, ViewTouch, WeakView},
};

/// Room between the line of the place and the panel.
const GAP: f32 = 2.0;

/// Where a view that floats at a place can go.
struct Sides {
    x:          f32,
    /// Its top when it lies under the line, and when it lies over it.
    below:      f32,
    above:      f32,
    room_below: bool,
    room_above: bool,
}

impl CodeEditor {
    /// Shows `content` at the place `pos` of the code: its left edge at the
    /// char, under the line, and over the line when the window has no room
    /// below. It slides sideways back inside the window. The content comes
    /// sized, like the one of a `Popover`, or sizes itself in its `setup`,
    /// like one that measures its text. It floats over the editor and
    /// the views around it and follows its place when the code scrolls.
    /// Only 1 panel shows, a new one takes the place of the old one. The
    /// app hides it with `hide_panel`.
    ///
    /// A panel shown for a rest of the pointer stays while the pointer is
    /// on it: `pointer_rested` tells the end of the rest only after the
    /// pointer left both the place and the panel. So a panel can hold a
    /// scroll view for a text longer than it, the wheel moves that text.
    pub fn show_panel(&self, pos: CodePos, content: Own<dyn View>) -> WeakView {
        let mut this = weak_from_ref(self);
        this.hide_panel();
        let (view, width, height) = float(content, "CodePanel");
        view.place().custom(move |rect: &mut Rect| {
            *rect = this.panel_frame(pos, width, height);
        });
        view.enable_hover();
        view.touch().hover_within.val(this, move |over| this.panel_hovered(over));
        this.panel = view;
        this.panel_at = pos;
        view
    }

    pub fn hide_panel(&self) {
        let mut this = weak_from_ref(self);
        let mut panel = take(&mut this.panel);
        if panel.is_ok() {
            panel.remove_from_superview();
        }
        this.panel_closed();
    }

    /// The panel on screen, null while none shows.
    pub fn panel(&self) -> WeakView {
        self.panel
    }

    /// Shows `content` over the line of the place `pos`, its left edge at
    /// the char, and under the line when the window has no room above.
    /// For what goes with the typing at a place, like the signature of
    /// the call whose arguments are typed. It leaves the room under the
    /// line to the panel, so a list of completions and a note show at
    /// once. Where both need the same side of 1 line, the panel has it
    /// and the note does not show. The rest is as with `show_panel`: 1
    /// note shows, a new one takes the place of the old one, and the app
    /// hides it with `hide_note`. Escape hides it too, before it drops a
    /// selection or ends the editing, and so does the editor when it
    /// loses the keys.
    pub fn show_note(&self, pos: CodePos, content: Own<dyn View>) -> WeakView {
        let mut this = weak_from_ref(self);
        this.hide_note();
        let (view, width, height) = float(content, "CodeNote");
        view.place().custom(move |rect: &mut Rect| {
            *rect = this.note_frame(pos, width, height);
        });
        this.note = view;
        view
    }

    pub fn hide_note(&self) {
        let mut this = weak_from_ref(self);
        let mut note = take(&mut this.note);
        if note.is_ok() {
            note.remove_from_superview();
        }
    }

    /// The note on screen, null while none shows.
    pub fn note(&self) -> WeakView {
        self.note
    }

    /// Where a view of this size can go at `pos`, in the window. `None`
    /// for an editor that is gone or hidden, like the one of another tab,
    /// and for a line that is not laid out.
    fn sides(self: Weak<Self>, pos: CodePos, width: f32, height: f32) -> Option<Sides> {
        if self.is_null() || self.is_hidden_in_tree() {
            return None;
        }
        let point = self.point_of(pos)?;
        let frame = self.absolute_frame();
        let screen = UIManager::root_view().frame().size;
        let half_row = self.line_height() / 2.0;
        let below = frame.y() + point.y + half_row + GAP;
        let above = frame.y() + point.y - half_row - GAP - height;
        Some(Sides {
            x:          (frame.x() + point.x).min(screen.width - width).max(0.0).round(),
            below:      below.round(),
            above:      above.round(),
            room_below: below + height <= screen.height,
            room_above: above >= 0.0,
        })
    }

    /// Where a panel of this size goes in the window. It has no size
    /// while it has no place.
    fn panel_frame(self: Weak<Self>, pos: CodePos, width: f32, height: f32) -> Rect {
        let Some(sides) = self.sides(pos, width, height) else {
            return Rect::default();
        };
        let y = if panel_goes_over(&sides) {
            sides.above
        } else {
            sides.below
        };
        Rect::new(sides.x, y, width, height)
    }

    /// Where a note of this size goes in the window. It has no size while
    /// it has no place, and while the panel lies on its side of its line.
    fn note_frame(self: Weak<Self>, pos: CodePos, width: f32, height: f32) -> Rect {
        let Some(sides) = self.sides(pos, width, height) else {
            return Rect::default();
        };
        let over = sides.room_above || !sides.room_below;
        if self.panel.is_ok() && self.panel_at.line == pos.line {
            let panel = self.panel.size();
            let panel_over = self
                .sides(self.panel_at, panel.width, panel.height)
                .is_some_and(|sides| panel_goes_over(&sides));
            if panel_over == over {
                return Rect::default();
            }
        }
        let y = if over { sides.above } else { sides.below };
        Rect::new(sides.x, y, width, height)
    }
}

/// A panel lies under its line, and over it only when that is the 1 side
/// with room.
fn panel_goes_over(sides: &Sides) -> bool {
    !sides.room_below && sides.room_above
}

/// Puts `content` over the whole window and gives it with its size.
fn float(content: Own<dyn View>, label: &str) -> (WeakView, f32, f32) {
    content.set_z_position(UIManager::TOAST_Z_OFFSET);
    let mut view = UIManager::root_view().add_subview_to_root(content);
    // Read after the add, the `setup` of the content ran by now.
    let size = view.size();
    view.set_label(label);
    (view, size.width, size.height)
}
