use std::mem::take;

use plat::Platform;

use crate::{
    deps::refs::{Own, Weak, main_lock::MainLock},
    gm::{Animation, flat::Point},
    ui::{
        Touch, TouchLock, TouchStack, UIAnimation, UIManager, View, ViewData, ViewFrame, ViewSubviews,
        WeakView, input::TouchEvent,
    },
    window::MouseButton,
};

/// How far from the left edge of the app views a swipe may start, in
/// points.
const EDGE: f32 = 24.0;
/// How far a finger moves before the swipe or the views take it.
const SLOP: f32 = 10.0;
/// A last step to the right of at least this many points finishes the
/// swipe also before half the width, a fling.
const FLING: f32 = 8.0;
/// Where the screen below starts, as a share of the width left of its
/// place, so it slides in slower than the finger, like on iOS.
const PARALLAX: f32 = 0.3;
/// The seconds of a slide across the whole width after the release.
const SLIDE: f32 = 0.3;

type Build = Box<dyn FnMut() -> Own<dyn View> + Send>;
type Done = Box<dyn FnMut() + Send>;

struct Entry {
    screen: WeakView,
    build:  Option<Build>,
    done:   Option<Done>,
}

#[derive(Default)]
enum Phase {
    #[default]
    Idle,
    /// A finger came down at the edge. It is a swipe once it moves right.
    Watching { id: usize, start: Point },
    /// The app views follow the finger, the screen below shows behind.
    Dragging {
        id:     usize,
        start:  f32,
        offset: f32,
        step:   f32,
        screen: WeakView,
    },
    /// The release slides the app views the rest of the way.
    Settling,
}

static ENTRIES: MainLock<Vec<Entry>> = MainLock::new();
static PHASE: MainLock<Phase> = MainLock::new();

/// A swipe from the left edge that leaves a screen, like the back swipe of
/// a native iOS app. The screen gives the function that builds the screen
/// it goes back to, the same one its back arrow and Escape call through
/// `UIManager::set_view`. When a finger starts at the edge and moves right,
/// that screen is built behind the current one, the current one follows
/// the finger, and a release past half the width or a fling finishes the
/// switch. Any other release slides the screen back and drops the one
/// below.
///
/// Only a screen set with `UIManager::set_view` swipes, and only with no
/// modal open. It works where a finger drags the content, see
/// `UIManager::drag_scrolling`, so on a phone and a tablet. The browser and
/// a TV keep the back arrow and Escape. The screen below is new, it has to
/// restore its own scroll place.
pub struct SwipeBack;

impl SwipeBack {
    /// `back` builds the screen below `screen`. It only builds, it may run
    /// for a swipe that goes back to its start.
    pub fn set<T: View + ?Sized>(screen: Weak<T>, back: impl FnMut() -> Own<dyn View> + Send + 'static) {
        Self::add(screen.weak_view(), Box::new(back), None);
    }

    /// Like `set`, and `done` runs once the screen below became the app
    /// views, never for a swipe that went back to its start. For what the
    /// back function does besides building, like turning the focus ring
    /// on again.
    pub fn set_with_done<T: View + ?Sized>(
        screen: Weak<T>,
        back: impl FnMut() -> Own<dyn View> + Send + 'static,
        done: impl FnMut() + Send + 'static,
    ) {
        Self::add(screen.weak_view(), Box::new(back), Some(Box::new(done)));
    }

    fn add(screen: WeakView, build: Build, done: Option<Done>) {
        let entries = ENTRIES.get_mut();
        entries.retain(|entry| entry.screen.is_ok() && entry.screen.raw() != screen.raw());
        entries.push(Entry {
            screen,
            build: Some(build),
            done,
        });
    }

    /// A new test, or a new screen set while a swipe ran.
    pub(crate) fn reset() {
        *PHASE.get_mut() = Phase::Idle;
    }

    /// Follows the finger. True when the touch belongs to a swipe and no
    /// view may see it. `touch` is in screen points.
    pub(crate) fn swallow(touch: &Touch) -> bool {
        match *PHASE.get_mut() {
            Phase::Idle => {
                if touch.is_began() && Self::may_start(touch) {
                    *PHASE.get_mut() = Phase::Watching {
                        id:    touch.id,
                        start: touch.position,
                    };
                }
                false
            }
            Phase::Watching { id, start } => {
                if touch.id != id {
                    return false;
                }
                Self::watch(touch, start);
                matches!(*PHASE.get_mut(), Phase::Dragging { .. })
            }
            Phase::Dragging { id, .. } => {
                // A second finger reaches no view of a moving screen.
                if touch.id == id {
                    Self::drag(touch);
                }
                true
            }
            Phase::Settling => true,
        }
    }

    fn may_start(touch: &Touch) -> bool {
        if touch.button != MouseButton::Left
            || Platform::WASM
            || Platform::TVOS
            || !UIManager::drag_scrolling()
            || UIManager::touch_disabled()
            || TouchStack::overlay_count() > 0
            || Self::current().is_none()
        {
            return false;
        }
        let layer = UIManager::root_view().screen_layer();
        let frame = layer.absolute_frame();
        touch.position.x - frame.origin.x <= EDGE
            && touch.position.y >= frame.origin.y
            && touch.position.y <= frame.max_y()
    }

    /// The entry of the screen on top of the app views now.
    fn current() -> Option<usize> {
        let layer = UIManager::root_view().screen_layer().raw();
        ENTRIES.get_mut().iter().rposition(|entry| {
            entry.screen.is_ok()
                && entry.build.is_some()
                && entry.screen.superview().raw() == layer
                && !entry.screen.is_hidden()
        })
    }

    fn watch(touch: &Touch, start: Point) {
        let moved = touch.position - start;
        match touch.event {
            TouchEvent::Moved if moved.x >= SLOP && moved.x > moved.y.abs() => Self::begin(touch, start),
            TouchEvent::Moved if moved.y.abs() < SLOP && moved.x > -SLOP => (),
            _ => Self::reset(),
        }
    }

    fn begin(touch: &Touch, start: Point) {
        let Some(index) = Self::current() else {
            Self::reset();
            return;
        };
        let screen = ENTRIES.get_mut()[index].screen;
        // The build can set the swipe of the screen it makes, so the list
        // is not held while it runs.
        let Some(mut build) = ENTRIES.get_mut()[index].build.take() else {
            Self::reset();
            return;
        };
        let below = build();
        if let Some(entry) = ENTRIES.get_mut().iter_mut().find(|entry| entry.screen.raw() == screen.raw()) {
            entry.build = Some(build);
        }

        // The views that caught the finger let it go, so the release is no
        // tap and no scroll.
        TouchStack::cancel_touch(touch.id);
        TouchStack::release_scrolls(touch.id);

        UIManager::root_view().add_under(below).place().back();
        log::debug!("swipe back began on {}", screen.label());

        *PHASE.get_mut() = Phase::Dragging {
            id: touch.id,
            start: start.x,
            offset: 0.0,
            step: 0.0,
            screen,
        };
        Self::drag(touch);
    }

    fn drag(touch: &Touch) {
        let Phase::Dragging {
            start,
            offset,
            screen,
            ..
        } = *PHASE.get_mut()
        else {
            return;
        };
        let width = Self::width();
        let now = (touch.position.x - start).clamp(0.0, width);
        if let Phase::Dragging {
            offset: stored, step, ..
        } = PHASE.get_mut()
        {
            // The release comes where the last move was, the step of a
            // fling is the one before it.
            if touch.is_moved() {
                *step = now - offset;
            }
            *stored = now;
        }
        Self::place(now, width);

        if touch.is_ended() {
            Self::release(screen);
        }
    }

    fn release(screen: WeakView) {
        let Phase::Dragging { offset, step, .. } = take(PHASE.get_mut()) else {
            return;
        };
        let width = Self::width();
        let finish = offset > width / 2.0 || step >= FLING;
        let target = if finish { width } else { 0.0 };
        log::debug!("swipe back released at {offset} of {width}, step {step}, finish {finish}");

        *PHASE.get_mut() = Phase::Settling;
        let distance = (target - offset).abs();
        if distance < 0.5 || width <= 0.0 {
            Self::complete(finish, screen);
            return;
        }

        let lock: TouchLock = Touch::lock();
        let layer = UIManager::root_view().screen_layer();
        let animation = UIAnimation::new(move |_, x| Self::place(x, width)).animation(Animation::new(
            offset,
            target,
            SLIDE * distance / width,
        ));
        let mut lock = Some(lock);
        animation.on_finish.sub(move || {
            drop(lock.take());
            Self::complete(finish, screen);
        });
        layer.add_animation(animation);
    }

    fn complete(finish: bool, screen: WeakView) {
        // A screen set while the slide ran has reset the root already.
        if !matches!(*PHASE.get_mut(), Phase::Settling) {
            return;
        }
        *PHASE.get_mut() = Phase::Idle;
        let root = UIManager::root_view();
        if !finish {
            root.clear_under();
            return;
        }

        let done = ENTRIES
            .get_mut()
            .iter_mut()
            .find(|entry| entry.screen.raw() == screen.raw())
            .and_then(|entry| entry.done.take());
        ENTRIES.get_mut().retain(|entry| entry.screen.raw() != screen.raw());
        root.raise_under();
        log::info!("swipe back left {}", screen.label());
        if let Some(mut done) = done {
            done();
        }
    }

    fn width() -> f32 {
        UIManager::root_view().screen_layer().width()
    }

    /// The app views `offset` points right of their place, the screen
    /// below a share of the rest left of its own.
    fn place(offset: f32, width: f32) {
        let behind = if width > 0.0 {
            -width * PARALLAX * (1.0 - offset / width)
        } else {
            0.0
        };
        UIManager::root_view().place_layers(offset, behind);
    }
}
