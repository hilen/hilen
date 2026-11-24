//! Where the pointer is over the code of a `CodeEditor`: a press at a
//! place, and a rest on a char, which an app answers with what it knows of
//! the name there. A rest holds while the pointer is on the panel the app
//! showed for it, so the user can scroll a long text in that panel.

use super::{text::CodePos, view::CodeEditor};
use crate::{
    deps::{hreads::after, refs::Weak},
    gm::flat::Point,
    ui::{UIEvents, ViewFrame, ViewTouch},
};

/// Seconds the pointer stays on 1 place before it counts as resting.
const REST_SECONDS: f32 = 0.4;
/// Seconds a rest still holds after the pointer left its place while a
/// panel shows, the time the pointer needs to get onto the panel.
const LEAVE_SECONDS: f32 = 0.3;

/// A press with a mouse that put the cursor at a place of the code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CodePress {
    pub pos:     CodePos,
    /// Cmd was held on a Mac, Ctrl elsewhere. The press started no
    /// selection then.
    pub command: bool,
}

/// What the editor knows of a pointer over its code.
#[derive(Debug, Default)]
pub(super) struct Rest {
    /// The place under the pointer, `None` off the code.
    at:       Option<CodePos>,
    /// The place `pointer_rested` told of, `None` once it told that the
    /// rest is over.
    told:     Option<CodePos>,
    /// Counts the moves to another place, only the wait of the newest one
    /// tells.
    moves:    u64,
    /// The pointer is on the panel of the editor.
    on_panel: bool,
    /// The rest ends soon, unless the pointer comes back to its place or
    /// goes onto the panel.
    leaving:  bool,
    /// Counts the waits before a rest ends, only the newest one ends it.
    leaves:   u64,
}

impl CodeEditor {
    /// The moves of the pointer come only while it is over the editor. A
    /// view gets its own `moved` only while it is pressed.
    pub(super) fn setup_pointer(self: Weak<Self>) {
        self.enable_hover();
        self.touch().hovered.val(self, move |over| {
            if over {
                UIEvents::pointer_moved().val(self, move |point| self.pointer_at(Some(point)));
            } else {
                UIEvents::pointer_moved().unsubscribe(self);
                self.pointer_at(None);
            }
        });
    }

    /// The pointer is at `point` of the window, or left the editor.
    fn pointer_at(mut self: Weak<Self>, point: Option<Point>) {
        if self.is_null() {
            return;
        }
        let at = point.and_then(|point| {
            let frame = self.absolute_frame();
            self.text_at(Point::new(point.x - frame.x(), point.y - frame.y()))
        });
        if at == self.rest.at {
            return;
        }
        self.rest.at = at;
        self.rest.moves += 1;
        match self.rest.told {
            Some(told) if at == Some(told) => self.stay(),
            Some(_) => self.leave_soon(),
            None => {}
        }
        if at.is_some() && at != self.rest.told {
            let moves = self.rest.moves;
            after(REST_SECONDS, move || self.rested(moves));
        }
    }

    /// The wait after the move number `moves` is over.
    fn rested(mut self: Weak<Self>, moves: u64) {
        if self.is_null() || moves != self.rest.moves || self.rest.on_panel {
            return;
        }
        if let Some(at) = self.rest.at {
            self.stay();
            self.rest.told = Some(at);
            self.pointer_rested.trigger(Some(at));
        }
    }

    /// The pointer came onto the panel of the editor or left it.
    pub(super) fn panel_hovered(mut self: Weak<Self>, over: bool) {
        if self.is_null() {
            return;
        }
        self.rest.on_panel = over;
        if over {
            self.stay();
            // The pointer crossed other places on its way, none of them
            // is a rest.
            self.rest.moves += 1;
        } else if self.rest.told.is_some() && self.rest.at != self.rest.told {
            self.leave_soon();
        }
    }

    /// The panel is gone, the pointer can not be on it.
    pub(super) fn panel_closed(mut self: Weak<Self>) {
        self.rest.on_panel = false;
    }

    /// The rest holds, a wait for its end is over.
    fn stay(mut self: Weak<Self>) {
        self.rest.leaving = false;
        self.rest.leaves += 1;
    }

    /// The pointer left the place of the rest. With no panel the rest is
    /// over at once. With one it holds for a moment: the pointer crosses
    /// other places on its way onto the panel.
    fn leave_soon(mut self: Weak<Self>) {
        if self.panel.is_null() {
            self.end_rest();
            return;
        }
        if self.rest.leaving {
            return;
        }
        self.rest.leaving = true;
        self.rest.leaves += 1;
        let leaves = self.rest.leaves;
        after(LEAVE_SECONDS, move || self.left(leaves));
    }

    /// The wait number `leaves` before the end of a rest is over.
    fn left(mut self: Weak<Self>, leaves: u64) {
        if self.is_null() || leaves != self.rest.leaves {
            return;
        }
        self.rest.leaving = false;
        if self.rest.on_panel || self.rest.at == self.rest.told {
            return;
        }
        self.end_rest();
    }

    fn end_rest(mut self: Weak<Self>) {
        if self.rest.told.take().is_some() {
            self.pointer_rested.trigger(None);
        }
    }
}
