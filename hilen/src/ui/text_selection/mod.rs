//! The text selection of the app. There is 1 at a time, like in a
//! browser page. It goes over a selectable `Label`, over the texts of a
//! selectable `MarkdownView`, or over every cell of a `TableView` with
//! selectable text. The ends are kept as positions in the data, see
//! `Position`, never as views.
//!
//! A view takes no event for it. The input code calls in here: a press a
//! target view took starts a selection, every touch after it moves the
//! free end, and the frame tick follows the pointer and scrolls a table
//! whose edge the drag reached.

mod menu;
mod pieces;
mod position;
mod words;

use std::collections::HashMap;

use anyhow::Result;
use log::{debug, warn};

pub(crate) use self::{pieces::texts, position::Lead, words::word_range};
use self::{
    pieces::{hit, is_target, pieces, scope_of},
    position::{Granularity, Position, Scope},
};
use crate::{
    deps::{
        hreads::on_main,
        refs::{Weak, main_lock::MainLock, weak_from_ref},
    },
    gm::{Clock, LossyConvert, flat::Point},
    system::Clipboard,
    ui::{
        Input, Label, Mouse, TableView, Touch, TouchStack, UIManager, View, ViewData, ViewFrame, WeakView,
        input::TouchEvent, view::DoubleTap,
    },
    window::{MouseButton, request_frame},
};

/// A drag this close to the top or the bottom edge of a table scrolls it.
const EDGE: f32 = 24.0;
/// Points a second the table scrolls per point the pointer is past the
/// start of the edge zone, and the bounds of that speed.
const SCROLL_GAIN: f32 = 14.0;
const MIN_SCROLL_SPEED: f32 = 60.0;
const MAX_SCROLL_SPEED: f32 = 2400.0;
/// A frame that took longer scrolls as if it took this long, so a stall
/// does not throw the list far away.
const MAX_TICK_MS: f64 = 50.0;

/// How far a press may move and still end as a tap.
const TAP_SLOP: f32 = 10.0;

/// A press on selectable text where a drag scrolls and does not select.
/// The text took the touch, so when the press ends where it began, it is
/// passed on as a tap on the row under it.
struct Tap {
    touch: usize,
    view:  WeakView,
    point: Point,
}

/// The press that moves the free end of the selection until it ends.
struct Drag {
    touch:   usize,
    /// Where the pointer is now, on the screen.
    pointer: Point,
    moved:   bool,
    /// The view that took the press.
    pressed: WeakView,
    /// Started by a long press on a touch screen. Its release opens the
    /// menu with Copy, a finger has no other way to it.
    by_hold: bool,
}

struct State {
    scope:       Scope,
    /// What the first click took, the selection never gets smaller than it.
    anchor:      (Position, Position),
    /// The selected text, start before end.
    range:       (Position, Position),
    granularity: Granularity,
    drag:        Option<Drag>,
}

impl State {
    fn is_empty(&self) -> bool {
        self.range.0 >= self.range.1
    }

    fn take(&mut self, unit: (Position, Position)) {
        self.range = (self.anchor.0.min(unit.0), self.anchor.1.max(unit.1));
    }
}

/// Which row and piece every selectable label on screen is, found once a
/// frame. The drawer asks for every label it draws.
#[derive(Default)]
struct Shown {
    valid: bool,
    /// By the address of the label.
    place: HashMap<usize, (usize, usize)>,
}

/// Tells a double and a triple click from single ones.
#[derive(Default)]
struct Clicks {
    taps:  DoubleTap,
    count: usize,
}

static STATE: MainLock<Option<State>> = MainLock::new();
static SHOWN: MainLock<Shown> = MainLock::new();
static CLICKS: MainLock<Clicks> = MainLock::new();
static TAP: MainLock<Option<Tap>> = MainLock::new();
static LAST_TICK_MS: MainLock<f64> = MainLock::new();

fn state() -> Option<&'static mut State> {
    let state = STATE.get_mut();
    if state.as_ref().is_some_and(|state| !state.scope.is_ok()) {
        *state = None;
    }
    state.as_mut()
}

fn changed() {
    SHOWN.get_mut().valid = false;
    request_frame();
}

/// The selected part of a label as the drawer needs it.
pub(crate) struct Highlight {
    /// Bytes of the full text of the label.
    pub start:   usize,
    pub end:     usize,
    /// The selection goes on after this label.
    pub goes_on: bool,
}

/// The text selection of the app, see `Label::set_selectable`,
/// `MarkdownView::set_selectable` and `TableView::set_text_selectable`.
pub struct TextSelection;

impl TextSelection {
    /// The selected text as it is drawn, with 1 line break between 2
    /// views. Empty when nothing is selected. A selection over a table
    /// sets up a cell for every selected row that is not on screen.
    pub fn text() -> String {
        let Some(state) = state() else {
            return String::new();
        };
        if state.is_empty() {
            return String::new();
        }
        let (start, end) = state.range;
        let mut text = String::new();

        match state.scope {
            Scope::View(view) => join(&mut text, 0, &texts(view), start, end),
            Scope::Table(mut table) => {
                let rows = table.selectable_rows();
                for row in start.row..rows.min(end.row.saturating_add(1)) {
                    join(&mut text, row, &table.row_texts(row), start, end);
                }
            }
        }
        text
    }

    /// Nothing is selected.
    pub fn is_empty() -> bool {
        state().is_none_or(|state| state.is_empty())
    }

    /// Drops the selection.
    pub fn clear() {
        if STATE.get_mut().take().is_some() {
            changed();
        }
    }

    /// Puts the selected text on the clipboard. Does nothing when nothing
    /// is selected, the clipboard keeps what it holds.
    pub fn copy() -> Result<()> {
        let text = Self::text();
        if text.is_empty() {
            return Ok(());
        }
        debug!("Copied {} bytes of selected text", text.len());
        Clipboard::set_text(text)
    }

    /// Takes all the text of the view the selection is in, the one the
    /// last click or long press went to. In a table this is the text of
    /// every row.
    pub fn select_all() {
        let Some(state) = state() else {
            return;
        };
        state.anchor = (Position::default(), Position::END);
        state.range = state.anchor;
        state.granularity = Granularity::Char;
        changed();
    }

    fn copy_logged() {
        if let Err(err) = Self::copy() {
            warn!("Failed to copy the selected text: {err}");
        }
    }
}

/// Adds the selected part of every text of `row` to `out`.
fn join(out: &mut String, row: usize, parts: &[(String, Lead)], start: Position, end: Position) {
    let mut first_of_row = true;

    for (piece, (text, lead)) in parts.iter().enumerate() {
        let key = (row, piece);
        if key < start.piece_key() || key > end.piece_key() {
            continue;
        }
        let from = if key == start.piece_key() {
            text.floor_char_boundary(start.byte.min(text.len()))
        } else {
            0
        };
        let to = if key == end.piece_key() {
            text.floor_char_boundary(end.byte.min(text.len()))
        } else {
            text.len()
        };
        if from > to {
            continue;
        }

        if !out.is_empty() {
            out.push_str(if first_of_row { Lead::Line } else { *lead }.as_str());
        }
        out.push_str(&text[from..to]);
        first_of_row = false;
    }
}

// What the input code calls.
impl TextSelection {
    /// Every touch of the app, before a view gets it. `touch.position`
    /// is in points on the screen.
    pub(crate) fn on_touch(touch: &Touch) {
        if touch.button != MouseButton::Left {
            return;
        }
        match touch.event {
            TouchEvent::Began => *TAP.get_mut() = None,
            TouchEvent::Moved => {}
            TouchEvent::Ended => {
                if let Some(tap) = TAP.get_mut().take_if(|tap| tap.touch == touch.id) {
                    Self::tapped(&tap, touch.position);
                }
            }
        }
        let Some(state) = state() else {
            return;
        };

        match touch.event {
            TouchEvent::Began => {
                state.drag = None;
                // A press somewhere else drops the selection, like a click
                // beside the text of an editor. A press on a menu or a
                // dialog over the scope leaves it, Copy of the menu still
                // has to find it.
                let view = state.scope.view();
                if TouchStack::key_reaches(view.raw()) && !view.absolute_frame().contains(touch.position) {
                    Self::clear();
                }
            }
            TouchEvent::Moved => {
                if let Some(drag) = &mut state.drag
                    && drag.touch == touch.id
                {
                    drag.pointer = touch.position;
                    drag.moved = true;
                }
            }
            TouchEvent::Ended => {
                if state.drag.as_ref().is_some_and(|drag| drag.touch == touch.id) {
                    Self::finish(touch.position);
                }
            }
        }
    }

    /// A view took a press of the left button or of a finger.
    pub(crate) fn pressed(view: WeakView, point: Point, touch: usize) {
        if !is_target(view) {
            return;
        }
        // Where a drag scrolls, on a touch screen, it never selects. A
        // tap there drops the selection and a long press starts one.
        if UIManager::drag_scrolling() {
            Self::clear();
            *TAP.get_mut() = Some(Tap { touch, view, point });
            return;
        }
        let Some(scope) = scope_of(view) else {
            return;
        };
        let Some(hit) = hit(scope, point) else {
            return;
        };

        let clicks = CLICKS.get_mut();
        clicks.count = if clicks.taps.tap_in_row(point) {
            clicks.count + 1
        } else {
            1
        };
        let granularity = match clicks.count {
            1 => Granularity::Char,
            2 => Granularity::Word,
            _ => Granularity::Line,
        };
        let unit = hit.unit(granularity);
        let drag = Some(Drag {
            touch,
            pointer: point,
            moved: false,
            pressed: view,
            by_hold: false,
        });

        let extends = Input::modifiers().shift_key();
        match state() {
            Some(state) if extends && state.scope.same(&scope) => {
                state.granularity = Granularity::Char;
                state.take(unit);
                state.drag = drag;
            }
            _ => {
                *STATE.get_mut() = Some(State {
                    scope,
                    anchor: unit,
                    range: unit,
                    granularity,
                    drag,
                });
            }
        }
        changed();
    }

    /// A press was held on `view` for the time of a long press. True
    /// when the selection took the hold, the view then gets no
    /// `secondary` event for it.
    pub(crate) fn held(view: WeakView, point: Point, touch: usize) -> bool {
        if !is_target(view) {
            return false;
        }
        *TAP.get_mut() = None;
        // A mouse that rests in the middle of a drag goes on with it.
        if state().is_some_and(|state| state.drag.as_ref().is_some_and(|drag| drag.touch == touch)) {
            return true;
        }
        let Some(scope) = scope_of(view) else {
            return false;
        };
        let Some(hit) = hit(scope, point) else {
            return false;
        };

        let unit = hit.unit(Granularity::Word);
        *STATE.get_mut() = Some(State {
            scope,
            anchor: unit,
            range: unit,
            granularity: Granularity::Word,
            drag: Some(Drag {
                touch,
                pointer: point,
                moved: false,
                pressed: view,
                by_hold: true,
            }),
        });
        // The finger now moves the end of the selection, not the list.
        TouchStack::release_scrolls(touch);
        changed();
        true
    }

    /// A press that selected nothing ended at `point`.
    fn tapped(tap: &Tap, point: Point) {
        if (point - tap.point).length() > TAP_SLOP || tap.view.is_null() {
            return;
        }
        if let Some(Scope::Table(table)) = scope_of(tap.view) {
            Self::tap_row(table, tap.view, point);
        }
    }

    /// A tap on the row at `point`, unless the table took the press
    /// itself and so sees the tap without help.
    fn tap_row(table: Weak<TableView>, pressed: WeakView, point: Point) {
        if pressed.is_ok() && pressed.addr() != table.weak_view().addr() {
            table.tap_at(point);
        }
    }

    /// A right click on `view`, at a point on the screen.
    pub(crate) fn right_clicked(view: WeakView, point: Point) {
        if !is_target(view) {
            return;
        }
        let Some(scope) = scope_of(view) else {
            return;
        };
        // A right click in a view with no selection yet still makes it the
        // one Select all works on.
        if state().is_none_or(|state| !state.scope.same(&scope)) {
            let at = hit(scope, point).map_or_else(Position::default, |hit| hit.position);
            *STATE.get_mut() = Some(State {
                scope,
                anchor: (at, at),
                range: (at, at),
                granularity: Granularity::Char,
                drag: None,
            });
            changed();
        }
        menu::show(point);
    }

    /// A typed character, with the modifiers held now.
    pub(crate) fn on_char(ch: char) {
        // A text field in edit has the keys, its own copy and select all.
        if !Input::command_held() || UIManager::text_editing() {
            return;
        }
        let Some(state) = state() else {
            return;
        };
        // Not a view under a dialog, and not one on a page that is not shown.
        let view = state.scope.view();
        if view.is_hidden_in_tree() || !TouchStack::key_reaches(view.raw()) {
            return;
        }
        match ch.to_ascii_lowercase() {
            'a' => Self::select_all(),
            'c' => Self::copy_logged(),
            _ => {}
        }
    }

    /// Once a frame, before the views update. Follows the pointer of a
    /// drag and scrolls a table whose edge it reached. The pointer can
    /// rest there with no touch event, so this cannot hang on events.
    pub(crate) fn tick() {
        SHOWN.get_mut().valid = false;

        let Some(state) = state() else {
            return;
        };
        let scope = state.scope;
        let Some(drag) = &state.drag else {
            return;
        };
        // The release never came, the window lost the pointer.
        if !Mouse::held(MouseButton::Left) {
            state.drag = None;
            return;
        }
        if !drag.moved {
            return;
        }
        let pointer = drag.pointer;

        // Read only while a drag runs. The first tick of a drag then sees a
        // long time since the last one, the bound below holds it.
        let now = Clock::now_ms();
        let last = LAST_TICK_MS.get_mut();
        let passed = (now - *last).clamp(0.0, MAX_TICK_MS);
        *last = now;

        Self::follow(pointer);

        let Scope::Table(mut table) = scope else {
            return;
        };
        let frame = *table.absolute_frame();
        let past_top = frame.y() + EDGE - pointer.y;
        let past_bottom = pointer.y - (frame.max_y() - EDGE);
        let speed = |past: f32| (past * SCROLL_GAIN).clamp(MIN_SCROLL_SPEED, MAX_SCROLL_SPEED);
        let seconds: f32 = (passed / 1000.0).lossy_convert();

        let offset = table.content_offset();
        let target = if past_top > 0.0 {
            offset + speed(past_top) * seconds
        } else if past_bottom > 0.0 {
            offset - speed(past_bottom) * seconds
        } else {
            return;
        };
        table.set_content_offset(target);
        // The pointer may rest, the next frame has to come by itself.
        request_frame();
    }

    /// Moves the free end of the selection to the text under `pointer`.
    fn follow(pointer: Point) {
        let Some(state) = state() else {
            return;
        };
        // Past the edge of a table the end stays on the row at that edge.
        let frame = *state.scope.view().absolute_frame();
        let point = match state.scope {
            Scope::View(_) => pointer,
            Scope::Table(_) => Point::new(
                pointer.x,
                pointer.y.clamp(frame.y() + 1.0, (frame.max_y() - 1.0).max(frame.y() + 1.0)),
            ),
        };
        let Some(hit) = hit(state.scope, point) else {
            return;
        };
        let before = state.range;
        state.take(hit.unit(state.granularity));
        if state.range != before {
            changed();
        }
    }

    /// The press of the drag ended at `point`.
    fn finish(point: Point) {
        let Some(state) = state() else {
            return;
        };
        let Some(drag) = state.drag.take() else {
            return;
        };
        if drag.moved {
            Self::follow(point);
        }
        let Some(state) = state_ref() else {
            return;
        };

        if drag.by_hold {
            // After this touch is done, a menu takes the touches of the app
            // from the moment it opens.
            on_main(move || menu::show(point));
            return;
        }

        // A click that selected nothing is a tap on the row under it. The
        // label took the touch, so the table did not see the tap itself.
        if state.is_empty()
            && let Scope::Table(table) = state.scope
        {
            Self::tap_row(table, drag.pressed, point);
        }
    }

    /// The selected part of `label`, none when no selection touches it.
    pub(crate) fn highlight(label: &Label) -> Option<Highlight> {
        let state = state_ref()?;
        if state.is_empty() {
            return None;
        }

        let shown = SHOWN.get_mut();
        if !shown.valid {
            shown.valid = true;
            shown.place.clear();
            match state.scope {
                Scope::View(view) => {
                    for (piece, label) in pieces(view).into_iter().enumerate() {
                        shown.place.insert(label.addr(), (0, piece));
                    }
                }
                Scope::Table(table) => {
                    for (row, cell) in table.visible_cells() {
                        for (piece, label) in pieces(cell).into_iter().enumerate() {
                            shown.place.insert(label.addr(), (row, piece));
                        }
                    }
                }
            }
        }

        let key = *shown.place.get(&weak_from_ref(label).addr())?;
        let (start, end) = state.range;
        if key < start.piece_key() || key > end.piece_key() {
            return None;
        }

        let len = label.text().len();
        let from = if key == start.piece_key() {
            start.byte.min(len)
        } else {
            0
        };
        let goes_on = key < end.piece_key();
        let to = if goes_on { len } else { end.byte.min(len) };

        (from < to || (from == to && goes_on)).then_some(Highlight {
            start: from,
            end: to,
            goes_on,
        })
    }
}

fn state_ref() -> Option<&'static State> {
    state().map(|state| &*state)
}
