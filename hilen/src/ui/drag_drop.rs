//! Drag and drop between views. A source view gives what a drag carries
//! and a picture that follows the pointer, a target view takes the drags
//! that carry its type. A mouse lifts a source once it moves 10 points with
//! the button down, also in a browser where a drag scrolls the page, a
//! finger after a hold of 0.4 seconds with no move, so a quick finger drag
//! still scrolls. The lift takes the touch from the views,
//! so the release is no tap. Escape puts the drag back. A scroll view under
//! the pointer scrolls while the pointer is near its edge. See
//! docs/drag-drop.md.

use std::{
    any::{Any, TypeId},
    cmp::Ordering::Equal,
    mem::{replace, take},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use log::debug;
use parking_lot::Mutex;
use plat::Platform;

use crate::{
    deps::{
        hreads::after,
        refs::{Own, Weak},
    },
    gm::flat::{Point, Size},
    ui::{
        LongPress, MouseButton, ScrollView, Shadow, Touch, TouchSource, TouchStack, UIManager, View,
        ViewData, ViewFrame, ViewSubviews, WeakView, view::depth_key,
    },
    window::slots::WindowLock,
};

/// How far a mouse moves with the button down before the source lifts, and
/// how far a finger may move before its hold no longer counts.
const SLOP: f32 = 10.0;
/// How long a finger holds a source still before it lifts.
const HOLD: f32 = 0.4;
/// How close to an edge of a scroll view the pointer scrolls it.
const EDGE: f32 = 48.0;
/// The most points a scroll view moves per step at the very edge.
const EDGE_SPEED: f32 = 14.0;
const STEP: f32 = 1.0 / 60.0;
/// The layer of the picture under the pointer, in front of a modal and
/// behind the focus ring and the debug views, see docs/depth.md.
const PICTURE_Z: f32 = 0.335;

/// What `Drag::source` gives when a drag starts.
pub struct DragStart<T> {
    payload: T,
    picture: Own<dyn View>,
    ended:   Option<Box<dyn FnOnce(bool) + Send>>,
}

impl<T> DragStart<T> {
    /// `picture` follows the pointer in front of everything, at the size and
    /// the place of the source when it lifted, until the drop.
    pub fn new(payload: T, picture: Own<dyn View>) -> Self {
        Self {
            payload,
            picture,
            ended: None,
        }
    }

    /// Runs when the drag ends, with true for a drop on a target and false
    /// when it went back, like a fade of the source that has to end.
    #[must_use]
    pub fn on_end(mut self, ended: impl FnOnce(bool) + Send + 'static) -> Self {
        self.ended = Some(Box::new(ended));
        self
    }
}

/// What a target hears of a drag that carries its type. The point is in
/// the target's own points.
#[derive(Clone, Debug, PartialEq)]
pub enum DropEvent<T> {
    /// The pointer came over the target.
    Enter(T, Point),
    /// The pointer moved over the target.
    Over(T, Point),
    /// The pointer left the target, or the drag went back.
    Leave,
    /// The drag was let go over the target.
    Drop(T, Point),
}

#[derive(Clone, Copy, Debug)]
enum DropKind {
    Enter,
    Over,
    Leave,
    Drop,
}

type StartAny = dyn FnMut() -> Option<Lifted> + Send;
type OnAny = dyn FnMut(&dyn Any, DropKind, Point) + Send;

struct Lifted {
    payload: Box<dyn Any + Send>,
    picture: Own<dyn View>,
    ended:   Option<Box<dyn FnOnce(bool) + Send>>,
}

#[derive(Clone)]
struct Source {
    view:  WeakView,
    start: Arc<Mutex<Box<StartAny>>>,
}

#[derive(Clone)]
struct Target {
    view: WeakView,
    /// The type of the payloads it takes.
    kind: TypeId,
    on:   Arc<Mutex<Box<OnAny>>>,
}

struct Held {
    id:      usize,
    payload: Box<dyn Any + Send>,
    picture: WeakView,
    /// Where the pointer holds the picture, from its top left corner.
    grab:    Point,
    ended:   Option<Box<dyn FnOnce(bool) + Send>>,
    over:    Option<Target>,
    finger:  Point,
    /// A step of the scroll at an edge is waiting.
    edge:    bool,
}

#[derive(Default)]
enum Phase {
    #[default]
    Idle,
    /// A press came down on a source. It lifts after a hold, or with a
    /// mouse once it moves.
    Watching {
        id:         usize,
        start:      Point,
        source:     Source,
        generation: u64,
        /// A mouse made the press, so a move lifts the source at once.
        mouse:      bool,
    },
    Dragging(Box<Held>),
}

static SOURCES: WindowLock<Vec<Source>> = WindowLock::new();
static TARGETS: WindowLock<Vec<Target>> = WindowLock::new();
static PHASE: WindowLock<Phase> = WindowLock::new();
static GENERATION: AtomicU64 = AtomicU64::new(0);

/// Drag and drop between views, see the module doc.
pub struct Drag;

impl Drag {
    /// `view` can be picked up. `start` runs when it lifts and gives what
    /// the drag carries and its picture, or `None` when this press starts
    /// no drag. A view has 1 start, a second call replaces it.
    pub fn source<V: View + ?Sized, T: Send + 'static>(
        view: Weak<V>,
        mut start: impl FnMut() -> Option<DragStart<T>> + Send + 'static,
    ) {
        let view = view.weak_view();
        let start: Box<StartAny> = Box::new(move || {
            start().map(|started| Lifted {
                payload: Box::new(started.payload),
                picture: started.picture,
                ended:   started.ended,
            })
        });
        let sources = SOURCES.get_mut();
        sources.retain(|source| source.view.is_ok() && source.view.raw() != view.raw());
        sources.push(Source {
            view,
            start: Arc::new(Mutex::new(start)),
        });
    }

    /// `view` takes the drags that carry a `T`. A view has 1 handler per
    /// type, a second call replaces it.
    pub fn target<V: View + ?Sized, T: Clone + 'static>(
        view: Weak<V>,
        mut on: impl FnMut(DropEvent<T>) + Send + 'static,
    ) {
        let view = view.weak_view();
        let on: Box<OnAny> = Box::new(move |payload, kind, point| {
            let Some(payload) = payload.downcast_ref::<T>() else {
                return;
            };
            on(match kind {
                DropKind::Enter => DropEvent::Enter(payload.clone(), point),
                DropKind::Over => DropEvent::Over(payload.clone(), point),
                DropKind::Leave => DropEvent::Leave,
                DropKind::Drop => DropEvent::Drop(payload.clone(), point),
            });
        });
        let kind = TypeId::of::<T>();
        let targets = TARGETS.get_mut();
        targets.retain(|target| {
            target.view.is_ok() && !(target.view.raw() == view.raw() && target.kind == kind)
        });
        targets.push(Target {
            view,
            kind,
            on: Arc::new(Mutex::new(on)),
        });
    }

    /// Takes a view out as a source and as a target.
    pub fn remove<V: View + ?Sized>(view: Weak<V>) {
        let view = view.weak_view();
        SOURCES.get_mut().retain(|source| source.view.raw() != view.raw());
        TARGETS.get_mut().retain(|target| target.view.raw() != view.raw());
    }

    /// A drag is held up right now.
    pub fn is_dragging() -> bool {
        matches!(*PHASE.get_mut(), Phase::Dragging(_))
    }

    /// A new test, a screen that went away, or another gesture took the
    /// touch. A held drag goes back.
    pub(crate) fn reset() {
        match take(PHASE.get_mut()) {
            Phase::Dragging(held) => end(*held, false),
            Phase::Idle | Phase::Watching { .. } => {}
        }
    }

    /// Escape puts a held drag back. True when one was held.
    #[cfg(feature = "gpu")]
    pub(crate) fn escape() -> bool {
        if !Self::is_dragging() {
            return false;
        }
        debug!("a drag went back on Escape");
        Self::reset();
        true
    }

    /// True when the touch belongs to a held drag and no view may see it.
    pub(crate) fn swallow(touch: &Touch) -> bool {
        if touch.is_began() && !Self::is_dragging() {
            *PHASE.get_mut() = watch_at(touch);
            return false;
        }
        match PHASE.get_mut() {
            Phase::Idle => false,
            Phase::Watching {
                id,
                start,
                source,
                generation,
                mouse,
            } => {
                if touch.id != *id {
                    return false;
                }
                let moved = (touch.position - *start).length();
                if touch.is_moved() && moved < SLOP {
                    return false;
                }
                if touch.is_moved() && *mouse {
                    let (id, start, source, generation) = (*id, *start, source.clone(), *generation);
                    lift(id, &source, start, generation);
                    if Self::is_dragging() {
                        drag(touch);
                        return true;
                    }
                    return false;
                }
                *PHASE.get_mut() = Phase::Idle;
                false
            }
            Phase::Dragging(held) => {
                // A second finger reaches no view while the first holds a drag.
                if touch.id == held.id {
                    drag(touch);
                }
                true
            }
        }
    }
}

/// The front source under a press that came down, in the top touch layer.
fn watch_at(touch: &Touch) -> Phase {
    if touch.button != MouseButton::Left || Platform::TVOS || UIManager::touch_disabled() {
        return Phase::Idle;
    }
    let top = TouchStack::top_layer_root();
    let sources = SOURCES.get_mut();
    sources.retain(|source| source.view.is_ok());
    let Some(source) = sources
        .iter()
        .filter(|source| shown_at(source.view, touch.position) && inside(source.view, top))
        .min_by(|a, b| depth_key(a.view).partial_cmp(&depth_key(b.view)).unwrap_or(Equal))
        .cloned()
    else {
        return Phase::Idle;
    };
    let generation = GENERATION.fetch_add(1, Ordering::Relaxed) + 1;
    let (id, start) = (touch.id, touch.position);
    let held = source.clone();
    after(HOLD, move || {
        if let Phase::Watching { generation: now, .. } = *PHASE.get_mut()
            && now == generation
        {
            lift(id, &held, start, generation);
        }
    });
    let mouse = match touch.source {
        TouchSource::Mouse => true,
        TouchSource::Finger => false,
        TouchSource::Unknown => !UIManager::drag_scrolling(),
    };
    Phase::Watching {
        id,
        start,
        source,
        generation,
        mouse,
    }
}

fn shown_at(view: WeakView, point: Point) -> bool {
    view.is_ok() && !view.is_hidden_in_tree() && view.contains_visible(point)
}

/// Whether `view` is `root` or in it.
fn inside(view: WeakView, root: WeakView) -> bool {
    let mut current = view;
    while current.is_ok() {
        if current.raw() == root.raw() {
            return true;
        }
        current = *current.superview();
    }
    false
}

fn lift(id: usize, source: &Source, finger: Point, generation: u64) {
    if !source.view.is_ok() {
        *PHASE.get_mut() = Phase::Idle;
        return;
    }
    let lifted = (source.start.lock())();
    let Some(lifted) = lifted else {
        *PHASE.get_mut() = Phase::Idle;
        return;
    };
    if !matches!(*PHASE.get_mut(), Phase::Watching { generation: now, .. } if now == generation) {
        return;
    }
    // The views and the scroll that caught the press let it go, so the
    // release is no tap, no hold and no scroll.
    TouchStack::cancel_touch(id);
    TouchStack::release_scrolls(id);
    LongPress::cancel(id);

    let frame = *source.view.absolute_frame();
    lifted.picture.set_z_position(PICTURE_Z);
    let mut root = UIManager::root_view();
    let picture = root.add_subview_to_root(lifted.picture);
    picture.set_shadow(Shadow::default());
    picture.set_frame(frame);
    debug!("a drag lifted {}", source.view.label());
    *PHASE.get_mut() = Phase::Dragging(Box::new(Held {
        id,
        payload: lifted.payload,
        picture,
        grab: finger - frame.origin,
        ended: lifted.ended,
        over: None,
        finger,
        edge: false,
    }));
    move_to(finger);
}

fn drag(touch: &Touch) {
    if touch.is_ended() {
        if let Phase::Dragging(held) = take(PHASE.get_mut()) {
            end(*held, true);
        }
        return;
    }
    if touch.is_moved() {
        move_to(touch.position);
    }
}

/// The picture under the pointer, and the target it is over told.
fn move_to(finger: Point) {
    let Phase::Dragging(held) = PHASE.get_mut() else {
        return;
    };
    held.finger = finger;
    let size: Size = held.picture.frame().size;
    let origin = finger - held.grab;
    held.picture.set_frame((origin.x, origin.y, size.width, size.height));
    let found = target_at(finger, &*held.payload);
    let before = held.over.clone();
    held.over.clone_from(&found);

    match (before, found) {
        (Some(old), Some(new)) if old.view.raw() == new.view.raw() => tell(&new, DropKind::Over, finger),
        (old, new) => {
            if let Some(old) = old {
                tell(&old, DropKind::Leave, finger);
            }
            if let Some(new) = new {
                tell(&new, DropKind::Enter, finger);
            }
        }
    }
    scroll_soon();
}

/// The front target under the pointer that takes the payload, in the top
/// touch layer.
fn target_at(finger: Point, payload: &dyn Any) -> Option<Target> {
    let top = TouchStack::top_layer_root();
    let targets = TARGETS.get_mut();
    targets.retain(|target| target.view.is_ok());
    targets
        .iter()
        .filter(|target| {
            payload.type_id() == target.kind && shown_at(target.view, finger) && inside(target.view, top)
        })
        .min_by(|a, b| depth_key(a.view).partial_cmp(&depth_key(b.view)).unwrap_or(Equal))
        .cloned()
}

/// Calls the handler of a target with the payload of the held drag. The
/// payload is taken out of the phase for the call, so the handler may
/// register views or read `Drag::is_dragging`.
fn tell(target: &Target, kind: DropKind, finger: Point) {
    let Phase::Dragging(held) = PHASE.get_mut() else {
        return;
    };
    let payload: Box<dyn Any + Send> = replace(&mut held.payload, Box::new(()));
    let local = target.view.__base_view().local_point(finger);
    (target.on.lock())(&*payload, kind, local);
    if let Phase::Dragging(held) = PHASE.get_mut() {
        held.payload = payload;
    }
}

/// The drag ends: a drop on the target under it when `drop` is true, back
/// otherwise. The picture goes away.
fn end(mut held: Held, drop: bool) {
    let dropped = drop && held.over.is_some();
    if let Some(over) = held.over.take() {
        let local = over.view.__base_view().local_point(held.finger);
        let kind = if dropped { DropKind::Drop } else { DropKind::Leave };
        (over.on.lock())(&*held.payload, kind, local);
    }
    if held.picture.is_ok() {
        held.picture.remove_from_superview();
    }
    debug!("a drag ended, {}", if dropped { "dropped" } else { "went back" });
    if let Some(ended) = held.ended.take() {
        ended(dropped);
    }
}

/// While the pointer rests near an edge of a scroll view around the target
/// under it, a timer moves that scroll view a step per frame. Only the timer
/// scrolls, so the speed does not follow how often the pointer moves.
fn scroll_soon() {
    let Phase::Dragging(held) = PHASE.get_mut() else {
        return;
    };
    let Some(over) = &held.over else {
        return;
    };
    if held.edge || !scroll_edges(over.view, held.finger, false) {
        return;
    }
    held.edge = true;
    let id = held.id;
    after(STEP, move || {
        let Phase::Dragging(held) = PHASE.get_mut() else {
            return;
        };
        if held.id != id {
            return;
        }
        held.edge = false;
        let finger = held.finger;
        let Some(over) = held.over.clone() else {
            return;
        };
        if scroll_edges(over.view, finger, true) {
            move_to(finger);
        }
    });
}

/// Whether a scroll view around `view` has the pointer near an edge it can
/// move away from. With `apply` each of them moves a step.
fn scroll_edges(view: WeakView, finger: Point, apply: bool) -> bool {
    let mut moved = false;
    let mut current = view;
    while current.is_ok() {
        if let Some(scroll) = current.downcast_view::<ScrollView>() {
            moved |= scroll_edge(scroll, finger, apply);
        }
        current = *current.superview();
    }
    moved
}

fn scroll_edge(scroll: Weak<ScrollView>, finger: Point, apply: bool) -> bool {
    let local = scroll.__base_view().local_point(finger);
    let size = scroll.size();
    let mut moved = false;

    let y = edge_step(local.y, size.height);
    if y != 0.0 {
        let before = scroll.get_scroll_content_offset();
        scroll.set_content_offset((before + y).min(0.0));
        moved |= (scroll.get_scroll_content_offset() - before).abs() > 0.01;
        if !apply {
            scroll.set_content_offset(before);
        }
    }
    let x = edge_step(local.x, size.width);
    if x != 0.0 {
        let before = scroll.content_offset_x();
        scroll.set_content_offset_x(before + x);
        moved |= (scroll.content_offset_x() - before).abs() > 0.01;
        if !apply {
            scroll.set_content_offset_x(before);
        }
    }
    moved
}

/// The step at a place along a side of `length`: into the content near the
/// start, out of it near the end, 0 away from both.
fn edge_step(at: f32, length: f32) -> f32 {
    if !(0.0..=length).contains(&at) {
        return 0.0;
    }
    if at < EDGE {
        EDGE_SPEED * (1.0 - at / EDGE)
    } else if at > length - EDGE {
        -EDGE_SPEED * (1.0 - (length - at) / EDGE)
    } else {
        0.0
    }
}
