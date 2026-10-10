//! Key focus, for a TV remote and for a keyboard. An arrow key moves a ring
//! to the nearest view that takes a tap in that direction, Enter taps the
//! view under the ring. An app writes nothing for it, the views that can
//! hold the ring are the touch views of the top touch layer, and every
//! visible cell of a table.

use ui_proc::view;

#[cfg(feature = "level")]
use crate::level::LevelManager;
#[cfg(feature = "scene")]
use crate::scene::SceneManager;
use crate::{
    deps::refs::{Weak, main_lock::MainLock},
    gm::{
        color::Color,
        flat::{Point, Rect},
    },
    ui::{
        Cursor, Input, NamedKey, Scrollable, Setup, TableView, Touch, TouchEvent, TouchStack, UIColor,
        UIManager, View, ViewData, ViewFrame, WeakView, view::ViewSubviews,
    },
    window::MouseButton,
};

/// The tap of the Enter key is a touch of its own finger, so a real
/// pointer that moves at the same moment never ends it.
pub(crate) const FOCUS_TOUCH_ID: usize = 0x00F0_C005;

const RING_WIDTH: f32 = 3.0;
const RING_GAP: f32 = 3.0;
const RING_COLOR: Color = Color::rgb(0.23, 0.56, 1.0);

/// How much a step off the axis of the move costs against a step along
/// it. A view straight ahead wins over a nearer one that sits diagonally.
const SIDE_COST: f32 = 4.0;

/// A table has no views past the rows it shows, and a scroll view clips
/// the rest. A move with nothing ahead scrolls by this much and looks
/// again on the next frame.
const SCROLL_STEP: f32 = 120.0;

/// A table makes the rows a scroll brings during the frames after it.
const PENDING_FRAMES: u8 = 4;

/// How many touch layers keep their ring place, more than any real stack
/// of modals is deep.
const REMEMBERED_LAYERS: usize = 8;

/// The space kept between a view the ring moved to and the edge of the
/// scroll view that clips it.
const REVEAL_MARGIN: f32 = 12.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FocusDirection {
    Left,
    Right,
    Up,
    Down,
}

impl FocusDirection {
    const fn index(self) -> usize {
        match self {
            Self::Left => 0,
            Self::Right => 1,
            Self::Up => 2,
            Self::Down => 3,
        }
    }

    fn of_key(key: NamedKey) -> Option<Self> {
        match key {
            NamedKey::ArrowLeft => Some(Self::Left),
            NamedKey::ArrowRight => Some(Self::Right),
            NamedKey::ArrowUp => Some(Self::Up),
            NamedKey::ArrowDown => Some(Self::Down),
            _ => None,
        }
    }
}

/// What a view keeps for the focus, in its `ViewBase`.
#[derive(Default)]
pub(crate) struct FocusData {
    skip:      bool,
    neighbors: [WeakView; 4],
}

#[derive(Clone, Copy)]
enum Target {
    View(WeakView),
    /// A table takes the taps of its cells itself and recycles the cell
    /// views, so a cell is known by its index.
    Cell {
        table: Weak<TableView>,
        index: usize,
    },
}

impl Target {
    fn same(self, other: Self) -> bool {
        match (self, other) {
            (Self::View(a), Self::View(b)) => a.raw() == b.raw(),
            (
                Self::Cell { table, index },
                Self::Cell {
                    table: other_table,
                    index: other_index,
                },
            ) => table.raw() == other_table.raw() && index == other_index,
            _ => false,
        }
    }

    /// The view the ring is drawn around, none when it is gone, hidden or
    /// under a layer that opened over it.
    fn view(self) -> Option<WeakView> {
        let view = match self {
            Self::View(view) => view,
            Self::Cell { table, index } => {
                if table.is_null() {
                    return None;
                }
                table.visible_cells().into_iter().find(|(cell, _)| *cell == index)?.1
            }
        };

        (view.is_ok() && !view.is_hidden_in_tree() && in_top_layer(view)).then_some(view)
    }
}

#[derive(Clone, Copy)]
struct Candidate {
    target: Target,
    rect:   Rect,
}

#[derive(Default)]
struct State {
    target:   Option<Target>,
    /// Where the ring was last, so a ring whose view died starts again
    /// from the same place.
    rect:     Rect,
    shown:    bool,
    disabled: bool,
    /// A move that scrolled and waits for the rows the scroll brings, with
    /// the frames it may still wait.
    pending:  Option<(FocusDirection, u8)>,
    ring:     Weak<FocusRing>,
    color:    Option<UIColor>,
    /// A view that moves a selection of its own with the arrows and
    /// holds the keys for it, a list of files.
    holder:   WeakView,
    /// Where the ring was in each touch layer, by the root of the layer.
    /// A modal that closes hands the ring back to the button that opened
    /// it.
    layers:   Vec<(WeakView, Target)>,
    /// Where the touch of a held Enter began, none while Enter is up.
    held:     Option<Point>,
}

static STATE: MainLock<State> = MainLock::new();

#[view]
struct FocusRing {}

impl Setup for FocusRing {
    fn setup(self: Weak<Self>) {
        self.set_border_width(RING_WIDTH);
    }
}

pub struct Focus;

impl Focus {
    /// Off means the arrow keys and Enter go to the app like any other
    /// key. A screen that uses the arrows itself, a video player that
    /// seeks with them, turns the focus off while it does.
    pub fn set_enabled(enabled: bool) {
        STATE.get_mut().disabled = !enabled;
        if !enabled {
            Self::hide();
        }
    }

    pub fn set_ring_color(color: impl Into<UIColor>) {
        STATE.get_mut().color = Some(color.into());
    }

    /// Puts the ring on `view` and shows it, the way a screen names where
    /// the remote starts.
    pub fn set(view: WeakView) {
        let state = STATE.get_mut();
        state.target = Some(Target::View(view));
        state.shown = true;
        state.pending = None;
        Self::place_ring();
    }

    /// Where the ring is drawn on the screen and how far it stands off its
    /// view, none while no ring shows.
    #[cfg(feature = "ui-tests")]
    pub(crate) fn ring_frame() -> Option<(Rect, f32)> {
        let ring = STATE.get_mut().ring;
        (ring.is_ok() && !ring.is_hidden()).then(|| (*ring.absolute_frame(), RING_WIDTH + RING_GAP))
    }

    /// The view under the ring, a null view while no ring shows.
    pub fn focused() -> WeakView {
        let state = STATE.get_mut();
        if !state.shown {
            return WeakView::default();
        }
        state.target.and_then(Target::view).unwrap_or_default()
    }

    pub fn hide() {
        let state = STATE.get_mut();
        state.shown = false;
        state.pending = None;
        if state.ring.is_ok() {
            state.ring.set_hidden(true);
        }
    }

    /// Gives the arrow keys and Enter to `view` until it lets go, is
    /// hidden or ends up under another touch layer. For a view that walks
    /// its own rows with the arrows, the ring would fight it for them.
    pub(crate) fn hold_keys(view: WeakView) {
        STATE.get_mut().holder = view;
        Self::hide();
    }

    pub(crate) fn release_keys(view: WeakView) {
        let state = STATE.get_mut();
        if state.holder.raw() == view.raw() {
            state.holder = WeakView::default();
        }
    }

    fn keys_held() -> bool {
        let holder = STATE.get_mut().holder;
        holder.is_ok() && !holder.is_hidden_in_tree() && in_top_layer(holder)
    }

    /// A new screen or a new test starts with no ring and no memory of
    /// the old one.
    pub(crate) fn reset() {
        let state = STATE.get_mut();
        Self::hide();
        state.holder = WeakView::default();
        state.target = None;
        state.layers.clear();
        state.rect = Rect::default();
        state.disabled = false;
        state.color = None;
    }

    /// A pointer that moves or presses takes over from the keys.
    pub(crate) fn pointer_used(touch: &Touch) {
        if touch.id != FOCUS_TOUCH_ID && STATE.get_mut().shown {
            Self::hide();
        }
    }

    /// True when the focus took the key, nothing else sees it then. A
    /// screen with nothing to put the ring on takes no key.
    pub(crate) fn on_key(key: NamedKey) -> bool {
        if !Self::active() {
            return false;
        }

        if let Some(direction) = FocusDirection::of_key(key) {
            return Self::go(direction, true);
        }

        if key == NamedKey::Enter {
            return Self::press();
        }

        false
    }

    /// Enter up ends the touch Enter down began. So Enter held for
    /// `LongPress::DURATION` is a long press and fires `secondary` on the
    /// view under the ring, and its release is then no tap, the same as a
    /// held finger.
    pub(crate) fn on_key_up(key: NamedKey) -> bool {
        if key != NamedKey::Enter {
            return false;
        }
        let Some(position) = STATE.get_mut().held.take() else {
            return false;
        };
        Self::touch(TouchEvent::Ended, position);
        true
    }

    /// After layout each frame. The ring follows its view, a move that
    /// scrolled looks again, and a ring whose view is gone, a modal
    /// opened over it, finds a new one.
    pub(crate) fn update() {
        let state = STATE.get_mut();
        if !state.shown {
            return;
        }

        if let Some((direction, frames)) = state.pending.take()
            && !Self::go(direction, false)
            && frames > 1
        {
            STATE.get_mut().pending = Some((direction, frames - 1));
        }

        let state = STATE.get_mut();
        if state.target.and_then(Target::view).is_none() {
            let root = TouchStack::top_layer_root().raw();
            let remembered = state
                .layers
                .iter()
                .find(|(layer, _)| layer.raw() == root)
                .map(|(_, target)| *target)
                .filter(|target| target.view().is_some());
            let from = state.rect;
            state.target =
                remembered.or_else(|| nearest_to(from, &candidates()).map(|candidate| candidate.target));
        }

        Self::place_ring();
    }

    /// Off by the app, while a text field is edited, while a game holds
    /// the mouse, and while a level or a scene runs that takes the arrow
    /// keys itself, see `LevelSetup::takes_keys`.
    fn active() -> bool {
        let free = !STATE.get_mut().disabled
            && !UIManager::text_editing()
            && !Cursor::captured()
            && !Self::keys_held();
        #[cfg(feature = "level")]
        let free = free && !LevelManager::takes_keys();
        #[cfg(feature = "scene")]
        let free = free && !SceneManager::takes_keys();
        free
    }

    fn go(direction: FocusDirection, may_scroll: bool) -> bool {
        let all = candidates();
        let state = STATE.get_mut();

        let current = state
            .target
            .and_then(|target| target.view().map(|view| (target, *view.absolute_frame())));

        // The first key only shows the ring, on the view it was on or on
        // the first one of the screen.
        let Some((target, rect)) = current.filter(|_| state.shown) else {
            let start = current.map(|(target, rect)| Candidate { target, rect });
            let Some(start) = start.or_else(|| nearest_to(state.rect, &all)) else {
                return false;
            };
            state.target = Some(start.target);
            state.shown = true;
            reveal(start.rect);
            Self::place_ring();
            return true;
        };

        if let Target::View(view) = target {
            let neighbor = view.__base_view().focus.neighbors[direction.index()];
            if neighbor.is_ok() && !neighbor.is_hidden_in_tree() {
                state.target = Some(Target::View(neighbor));
                reveal(*neighbor.absolute_frame());
                Self::place_ring();
                return true;
            }
        }

        let next = all
            .iter()
            .filter(|candidate| !candidate.target.same(target))
            .filter_map(|candidate| score(direction, rect, candidate.rect).map(|score| (score, candidate)))
            .min_by(|(a, _), (b, _)| a.total_cmp(b));

        if let Some((_, next)) = next {
            state.target = Some(next.target);
            reveal(next.rect);
            Self::place_ring();
            return true;
        }

        if !may_scroll {
            return false;
        }

        if let Some(mut scroll) = scroll_of(rect) {
            scroll.__scroll_by(step(direction));
            state.pending = Some((direction, PENDING_FRAMES));
        }

        true
    }

    fn press() -> bool {
        let state = STATE.get_mut();
        if !state.shown {
            return false;
        }
        // A key that is held repeats, and every repeat would tap again.
        if state.held.is_some() {
            return true;
        }
        let Some(view) = state.target.and_then(Target::view) else {
            return false;
        };

        let position = view.absolute_frame().center() * UIManager::scale();
        state.held = Some(position);
        Self::touch(TouchEvent::Began, position);

        true
    }

    fn touch(event: TouchEvent, position: Point) {
        Input::process_touch_event(Touch {
            id: FOCUS_TOUCH_ID,
            position,
            event,
            button: MouseButton::Left,
        });
    }

    fn place_ring() {
        let state = STATE.get_mut();
        let Some(view) = state.target.and_then(Target::view) else {
            if state.ring.is_ok() {
                state.ring.set_hidden(true);
            }
            return;
        };

        if state.ring.is_null() {
            let mut ring = FocusRing::new();
            ring.set_z_position(UIManager::MENU_Z_OFFSET - UIManager::subview_z_offset() * 3.0);
            let mut ring = UIManager::root_view().add_subview_to_root(ring);
            ring.set_label("Focus ring");
            state.ring = ring.downcast_view::<FocusRing>().expect("the focus ring is a FocusRing");
        }

        let rect = *view.absolute_frame();
        state.rect = rect;

        if let Some(target) = state.target {
            let root = TouchStack::top_layer_root();
            // The root of the first layer is the root view, which no
            // `Own` holds, so a layer is not asked whether it is alive.
            // The list stays short instead.
            state.layers.retain(|(layer, _)| layer.raw() != root.raw());
            if state.layers.len() >= REMEMBERED_LAYERS {
                state.layers.remove(0);
            }
            state.layers.push((root, target));
        }

        // The ring is one of the app views, and those start below the
        // status bar of a phone and move up with its keyboard. Its frame
        // is counted from where they start, not from the screen.
        let origin = UIManager::root_view_static().app_views_origin();
        let grow = RING_WIDTH + RING_GAP;
        let ring = state.ring;
        ring.set_hidden(false);
        ring.set_border_color(state.color.unwrap_or(UIColor::Plain(RING_COLOR)));
        ring.set_corner_radii(view.corner_radii());
        ring.set_frame((
            rect.x() - origin.x - grow,
            rect.y() - origin.y - grow,
            rect.width() + grow * 2.0,
            rect.height() + grow * 2.0,
        ));
    }
}

pub trait ViewFocus {
    /// Off keeps the ring away from a view that takes touches, a backdrop
    /// that covers the screen for one.
    fn set_key_focus(&self, on: bool) -> &Self;

    /// Names the view an arrow key goes to from this one, for a spot where
    /// the nearest view is the wrong one.
    fn set_focus_neighbor(&self, direction: FocusDirection, neighbor: WeakView) -> &Self;
}

impl<T: ?Sized + View> ViewFocus for T {
    fn set_key_focus(&self, on: bool) -> &Self {
        self.__base_view().focus.skip = !on;
        self
    }

    fn set_focus_neighbor(&self, direction: FocusDirection, neighbor: WeakView) -> &Self {
        self.__base_view().focus.neighbors[direction.index()] = neighbor;
        self
    }
}

/// True when `view` hangs in the tree under the root of the top touch
/// layer. A removed view keeps the link to its old parent until it is
/// freed a frame later, so each step up also asks the parent whether it
/// still holds the view.
fn in_top_layer(view: WeakView) -> bool {
    let root = TouchStack::top_layer_root().raw();
    let mut current = view;
    while current.is_ok() {
        if current.raw() == root {
            return true;
        }
        let parent = *current.superview();
        if parent.is_null() || !parent.subviews().iter().any(|sub| sub.raw() == current.raw()) {
            return false;
        }
        current = parent;
    }
    false
}

/// Every place the ring can go right now.
fn candidates() -> Vec<Candidate> {
    let root = TouchStack::top_layer_root().raw();
    let mut all = Vec::new();

    for view in TouchStack::touch_views() {
        if view.is_null() || view.raw() == root || view.__base_view().focus.skip {
            continue;
        }

        if let Some(table) = view.downcast_view::<TableView>() {
            if table.is_hidden_in_tree() {
                continue;
            }
            for (index, cell) in table.visible_cells() {
                if cell.is_visible_on_screen() {
                    all.push(Candidate {
                        target: Target::Cell { table, index },
                        rect:   *cell.absolute_frame(),
                    });
                }
            }
            continue;
        }

        let rect = *view.absolute_frame();
        if rect.width() <= 0.0 || rect.height() <= 0.0 || !view.is_visible_on_screen() {
            continue;
        }

        all.push(Candidate {
            target: Target::View(view),
            rect,
        });
    }

    all
}

/// The start of a ring with no view: the nearest one to where it was, on
/// a new screen that is the top left corner, so the first view in reading
/// order.
fn nearest_to(from: Rect, all: &[Candidate]) -> Option<Candidate> {
    let origin = from.center();
    all.iter().copied().min_by(|a, b| {
        let distance = |candidate: &Candidate| {
            let center = candidate.rect.center();
            (center.x - origin.x).abs() + (center.y - origin.y).abs() * 2.0
        };
        distance(a).total_cmp(&distance(b))
    })
}

/// How far `to` is from `from` for a move in `direction`, none when it is
/// not on that side at all. The gap along the move counts once and the
/// gap across it `SIDE_COST` times.
fn score(direction: FocusDirection, from: Rect, to: Rect) -> Option<f32> {
    let (along, across) = match direction {
        FocusDirection::Right => (
            ahead(from.x(), from.max_x(), to.x(), to.max_x())?,
            apart(from.y(), from.max_y(), to.y(), to.max_y()),
        ),
        FocusDirection::Left => (
            ahead(-from.max_x(), -from.x(), -to.max_x(), -to.x())?,
            apart(from.y(), from.max_y(), to.y(), to.max_y()),
        ),
        FocusDirection::Down => (
            ahead(from.y(), from.max_y(), to.y(), to.max_y())?,
            apart(from.x(), from.max_x(), to.x(), to.max_x()),
        ),
        FocusDirection::Up => (
            ahead(-from.max_y(), -from.y(), -to.max_y(), -to.y())?,
            apart(from.x(), from.max_x(), to.x(), to.max_x()),
        ),
    };

    let centers = match direction {
        FocusDirection::Left | FocusDirection::Right => (from.center().y - to.center().y).abs(),
        FocusDirection::Up | FocusDirection::Down => (from.center().x - to.center().x).abs(),
    };

    // The centers only break a tie between two views straight ahead.
    Some(along + across * SIDE_COST + centers * 0.01)
}

/// The gap from the span `from` to the span `to` along the move, none when
/// `to` does not reach past `from` or starts before its middle.
fn ahead(from_min: f32, from_max: f32, to_min: f32, to_max: f32) -> Option<f32> {
    let from_middle = f32::midpoint(from_min, from_max);
    let to_middle = f32::midpoint(to_min, to_max);
    (to_max > from_max && to_middle > from_middle).then(|| (to_min - from_max).max(0.0))
}

/// The gap between two spans across the move, zero when they overlap.
fn apart(a_min: f32, a_max: f32, b_min: f32, b_max: f32) -> f32 {
    (b_min - a_max).max(a_min - b_max).max(0.0)
}

/// The wheel delta that moves the content the way of the arrow. A wheel
/// delta above zero moves the content down, so an arrow down is below.
const fn step(direction: FocusDirection) -> Point {
    match direction {
        FocusDirection::Left => Point::new(SCROLL_STEP, 0.0),
        FocusDirection::Right => Point::new(-SCROLL_STEP, 0.0),
        FocusDirection::Up => Point::new(0.0, SCROLL_STEP),
        FocusDirection::Down => Point::new(0.0, -SCROLL_STEP),
    }
}

/// The scroll view a view at `rect` sits in, the innermost one. A view
/// half out of it has its middle outside, so the test is an overlap.
fn scroll_of(rect: Rect) -> Option<Weak<dyn Scrollable>> {
    TouchStack::scrolls()
        .filter(|scroll| scroll.is_ok() && !scroll.is_hidden_in_tree())
        .filter(|scroll| scroll.absolute_frame().intersects(&rect))
        .min_by(|a, b| area(a.absolute_frame()).total_cmp(&area(b.absolute_frame())))
}

fn area(rect: &Rect) -> f32 {
    rect.width() * rect.height()
}

/// Brings a view the ring moved to fully into the scroll view that clips
/// it.
fn reveal(rect: Rect) {
    let Some(mut scroll) = scroll_of(rect) else {
        return;
    };
    let clip = *scroll.absolute_frame();

    let mut delta = Point::default();

    if rect.y() < clip.y() + REVEAL_MARGIN {
        delta.y = clip.y() + REVEAL_MARGIN - rect.y();
    } else if rect.max_y() > clip.max_y() - REVEAL_MARGIN {
        delta.y = clip.max_y() - REVEAL_MARGIN - rect.max_y();
    }

    if rect.x() < clip.x() + REVEAL_MARGIN {
        delta.x = clip.x() + REVEAL_MARGIN - rect.x();
    } else if rect.max_x() > clip.max_x() - REVEAL_MARGIN {
        delta.x = clip.max_x() - REVEAL_MARGIN - rect.max_x();
    }

    if delta.x != 0.0 || delta.y != 0.0 {
        scroll.__scroll_by(delta);
    }
}

#[cfg(test)]
mod test {
    use super::{FocusDirection, apart, score};
    use crate::gm::flat::Rect;

    fn rect(x: f32, y: f32) -> Rect {
        Rect::new(x, y, 100.0, 50.0)
    }

    #[test]
    fn a_view_behind_or_level_is_not_ahead() {
        let from = rect(200.0, 200.0);
        assert!(score(FocusDirection::Right, from, rect(50.0, 200.0)).is_none());
        assert!(score(FocusDirection::Right, from, rect(200.0, 300.0)).is_none());
        assert!(score(FocusDirection::Left, from, rect(350.0, 200.0)).is_none());
        assert!(score(FocusDirection::Down, from, rect(200.0, 100.0)).is_none());
        assert!(score(FocusDirection::Up, from, rect(200.0, 300.0)).is_none());
    }

    #[test]
    fn straight_ahead_wins_over_a_nearer_diagonal() {
        let from = rect(200.0, 200.0);
        let straight = score(FocusDirection::Right, from, rect(500.0, 200.0)).unwrap();
        let diagonal = score(FocusDirection::Right, from, rect(310.0, 320.0)).unwrap();
        assert!(straight < diagonal);
    }

    #[test]
    fn the_nearer_of_two_views_ahead_wins() {
        let from = rect(200.0, 200.0);
        let near = score(FocusDirection::Down, from, rect(200.0, 260.0)).unwrap();
        let far = score(FocusDirection::Down, from, rect(200.0, 400.0)).unwrap();
        assert!(near < far);
    }

    #[test]
    fn overlapping_spans_are_not_apart() {
        assert!(apart(0.0, 10.0, 5.0, 20.0).abs() < f32::EPSILON);
        assert!((apart(0.0, 10.0, 15.0, 20.0) - 5.0).abs() < f32::EPSILON);
        assert!((apart(15.0, 20.0, 0.0, 10.0) - 5.0).abs() < f32::EPSILON);
    }
}
