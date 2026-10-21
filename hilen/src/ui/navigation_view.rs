use std::mem::take;

use plat::Platform;
use ui_proc::view;

use crate::{
    deps::{
        hreads::assert_main_thread,
        refs::{Own, Weak, main_lock::MainLock},
    },
    gm::{
        Animation,
        color::CLEAR,
        flat::{Point, Rect},
    },
    ui::{
        NamedKey, Setup, Touch, TouchLock, TouchStack, UIAnimation, UIManager, View, ViewData, ViewTouch,
        WeakView,
        input::TouchEvent,
        view::{ViewFrame, ViewSubviews},
    },
    window::MouseButton,
};

/// How far from the left edge of a stack a swipe may start, in points.
const EDGE: f32 = 24.0;
/// How far a finger moves before the swipe or the views take it.
const SLOP: f32 = 10.0;
/// A last step to the right of at least this many points finishes the
/// swipe also before half the width, a fling.
const FLING: f32 = 8.0;
/// Where the page below starts, as a share of the width left of its place,
/// so it slides in slower than the finger, like on iOS.
const PARALLAX: f32 = 0.3;
/// The seconds of a slide across the whole width after the release.
const SLIDE: f32 = 0.3;

/// A stack of pages. `push` puts a page on top, and every way back pops
/// it: a tap on its `BackButton`, a swipe from the left edge, Escape and
/// the Back key of Android or a remote. The page below stays alive and
/// hidden, so a pop shows it as it was. No page can have a way back that
/// does not swipe, the stack owns all of them. A push and a pop are
/// instant, only the swipe moves the pages, under the finger and after its
/// release.
#[view]
pub struct NavigationView {
    first_view: Option<Own<dyn View>>,
}

/// Holds 1 page of a stack. While a swipe shows the page under the top
/// one, the holder of that page is a flat layer, so the top
/// page drawn after it covers all of it. A later sibling draws behind the
/// subviews of an earlier one otherwise. A flat layer has to clip.
#[view]
struct PageHolder {
    covered: bool,
    /// How far right of its place a swipe moved it. The layout keeps it, a
    /// page placed with `back` alone would jump back.
    shift:   f32,
}

impl Setup for PageHolder {
    fn clips_to_bounds(&self) -> bool {
        self.covered
    }

    fn setup(self: Weak<Self>) {}
}

impl PageHolder {
    fn set_covered(mut self: Weak<Self>, covered: bool) {
        self.covered = covered;
        self.__base_view().flat_depth = covered;
    }

    fn set_shift(mut self: Weak<Self>, shift: f32) {
        self.shift = shift;
        self.set_x(shift);
    }

    fn page(&self) -> WeakView {
        self.subviews().first().map_or_else(WeakView::default, |page| page.weak_view())
    }

    /// The holder and its page, a page checks its own `is_hidden`.
    fn show(self: Weak<Self>, shown: bool) {
        self.set_hidden(!shown);
        let page = self.page();
        if page.is_ok() {
            page.set_hidden(!shown);
        }
    }
}

/// The way back of a pushed page. The stack makes it, the page asks for it
/// with `back_button()` and places it. Its look is free: a color, a corner
/// radius, and any views inside it, like an icon or a text, as long as they
/// take no touches. A tap on it pops the page.
#[view]
pub struct BackButton {
    nav: Weak<NavigationView>,
}

impl Setup for BackButton {
    fn setup(self: Weak<Self>) {
        assert!(
            self.nav.is_ok(),
            "a BackButton is made only by its NavigationView, get it with back_button() in the pushed page"
        );
        self.enable_touch();
        self.touch().up_inside.sub(self, move || {
            log::info!("back button of {}", self.superview().label());
            self.nav.pop();
        });
    }
}

#[derive(Default)]
enum Phase {
    #[default]
    Idle,
    /// A finger came down at the edge. It is a swipe once it moves right.
    Watching {
        id:    usize,
        start: Point,
        nav:   Weak<NavigationView>,
    },
    /// The top page follows the finger, the page below shows behind.
    Dragging {
        id:     usize,
        start:  f32,
        offset: f32,
        step:   f32,
        nav:    Weak<NavigationView>,
    },
    /// The release slides the top page the rest of the way.
    Settling,
}

static STACKS: MainLock<Vec<Weak<NavigationView>>> = MainLock::new();
static PHASE: MainLock<Phase> = MainLock::new();

impl NavigationView {
    pub fn with_view(first_view: Own<dyn View>) -> Own<Self> {
        Self {
            first_view: Some(first_view),
            ..Default::default()
        }
        .into()
    }
}

impl Setup for NavigationView {
    fn setup(mut self: Weak<Self>) {
        let view = self.first_view.take().expect("a NavigationView is made with with_view");
        self.add_page(view);
        let stacks = STACKS.get_mut();
        stacks.retain(|nav| nav.is_ok() && nav.raw() != self.raw());
        stacks.push(self);
    }
}

impl NavigationView {
    pub fn push(self: Weak<Self>, view: Own<dyn View>) {
        assert_main_thread();
        assert!(!self.subviews().is_empty(), "BUG: push from empty navigation");

        let below = self.top();
        let holder = self.add_page(view);
        below.show(false);
        log::info!("push {}", holder.page().label());

        // Escape, and the Back key of Android or a remote, leave the pushed
        // page. Only the page on top answers, a modal over it takes the key
        // first, and a swipe that still slides is not cut short.
        UIManager::keymap().add(holder, NamedKey::Escape, move || {
            let on_top = self.top().raw() == holder.raw();
            if on_top && !UIManager::text_editing() && !UIManager::touch_disabled() {
                self.pop();
            }
        });
    }

    pub fn pop(self: Weak<Self>) {
        assert_main_thread();
        assert!(self.subviews().len() > 1, "BUG: Nowhere to pop");

        let to_pop = self.top();
        log::info!("pop {}", to_pop.page().label());
        self.below().show(true);
        self.drop_top(to_pop);
    }

    /// The back button of the page that holds `view`. A page asks for it
    /// with `back_button()` of `ViewController`.
    pub(crate) fn back_button_of(self: Weak<Self>, view: WeakView) -> Weak<BackButton> {
        let mut page = view;
        while page.superview().is_ok() && page.superview().superview().raw() != self.raw() {
            page = *page.superview();
        }
        let index = self.subviews().iter().position(|holder| holder.raw() == page.superview().raw());
        let Some(index) = index else {
            panic!("{} is not in a page of this navigation stack", view.label());
        };
        assert!(
            index > 0,
            "{} is the first page of its stack, it has no page to go back to",
            page.label()
        );
        if let Some(button) = page.subviews().iter().find_map(|sub| sub.weak_view().downcast::<BackButton>())
        {
            return button;
        }
        let mut button = BackButton::new();
        button.nav = self;
        page.add_subview(button)
    }

    fn add_page(self: Weak<Self>, mut view: Own<dyn View>) -> Weak<PageHolder> {
        let mut holder = PageHolder::new();
        holder.set_navigation_view(self);
        let holder = self.add_subview(holder);
        holder
            .place()
            .back()
            .custom(move |rect: &mut Rect| rect.origin.x += holder.shift);
        view.set_navigation_view(self);
        let view = holder.add_subview(view);
        view.place().back();
        holder
    }

    fn top(&self) -> Weak<PageHolder> {
        self.holder(self.subviews().len() - 1)
    }

    fn below(&self) -> Weak<PageHolder> {
        self.holder(self.subviews().len() - 2)
    }

    fn holder(&self, index: usize) -> Weak<PageHolder> {
        self.subviews()[index]
            .weak_view()
            .downcast::<PageHolder>()
            .expect("a page of a navigation stack sits in a holder")
    }

    /// The top page is gone, the page below is the top one, in its place.
    fn drop_top(self: Weak<Self>, mut top: Weak<PageHolder>) {
        let below = self.below();
        below.set_covered(false);
        below.set_shift(0.0);
        top.remove_from_superview();
    }
}

/// The swipe from the left edge of a stack, like the back swipe of a native
/// iOS app. When a finger starts at the edge and moves right, the page below
/// shows behind the top one, the top one follows the finger, and a release
/// past half the width or a fling pops it. Any other release slides it back.
///
/// It runs where a finger drags the content, see `UIManager::drag_scrolling`,
/// so on a phone and a tablet. The browser and a TV keep the back button and
/// Escape. The innermost stack under the finger takes it, and only a stack in
/// the top touch layer, so not one under an open modal.
impl NavigationView {
    /// A new test, or a new screen set while a swipe ran.
    pub(crate) fn reset_swipe() {
        *PHASE.get_mut() = Phase::Idle;
    }

    /// Follows the finger. True when the touch belongs to a swipe and no
    /// view may see it. `touch` is in screen points.
    pub(crate) fn swallow(touch: &Touch) -> bool {
        match *PHASE.get_mut() {
            Phase::Idle => {
                if touch.is_began()
                    && let Some(nav) = Self::stack_at(touch)
                {
                    *PHASE.get_mut() = Phase::Watching {
                        id: touch.id,
                        start: touch.position,
                        nav,
                    };
                }
                false
            }
            Phase::Watching { id, start, nav } => {
                if touch.id != id {
                    return false;
                }
                Self::watch(touch, start, nav);
                matches!(*PHASE.get_mut(), Phase::Dragging { .. })
            }
            Phase::Dragging { id, .. } => {
                // A second finger reaches no view of a moving page.
                if touch.id == id {
                    Self::drag(touch);
                }
                true
            }
            Phase::Settling => true,
        }
    }

    /// The innermost stack with a page to go back to, whose left edge is
    /// under the finger.
    fn stack_at(touch: &Touch) -> Option<Weak<Self>> {
        if touch.button != MouseButton::Left
            || Platform::WASM
            || Platform::TVOS
            || !UIManager::drag_scrolling()
            || UIManager::touch_disabled()
        {
            return None;
        }
        let stacks = STACKS.get_mut();
        stacks.retain(Weak::is_ok);
        // A stack inside another one was set up after it.
        stacks.iter().rev().copied().find(|nav| {
            if nav.subviews().len() < 2 || nav.is_hidden_in_tree() || !TouchStack::key_reaches(nav.raw()) {
                return false;
            }
            let frame = nav.absolute_frame();
            touch.position.x >= frame.origin.x
                && touch.position.x - frame.origin.x <= EDGE
                && touch.position.y >= frame.origin.y
                && touch.position.y <= frame.max_y()
        })
    }

    fn watch(touch: &Touch, start: Point, nav: Weak<Self>) {
        let moved = touch.position - start;
        match touch.event {
            TouchEvent::Moved if moved.x >= SLOP && moved.x > moved.y.abs() => Self::begin(touch, start, nav),
            TouchEvent::Moved if moved.y.abs() < SLOP && moved.x > -SLOP => (),
            _ => Self::reset_swipe(),
        }
    }

    fn begin(touch: &Touch, start: Point, nav: Weak<Self>) {
        if !nav.is_ok() || nav.subviews().len() < 2 {
            Self::reset_swipe();
            return;
        }

        // The views that caught the finger let it go, so the release is no
        // tap and no scroll.
        TouchStack::cancel_touch(touch.id);
        TouchStack::release_scrolls(touch.id);

        let below = nav.below();
        below.show(true);
        below.set_covered(true);
        // A page with no background of its own shows the clear color of the
        // window. Under it the page below shows now, so the holder paints
        // that color while the swipe runs.
        nav.top().set_color(UIManager::clear_color());
        log::debug!("swipe back began on {}", nav.top().page().label());

        *PHASE.get_mut() = Phase::Dragging {
            id: touch.id,
            start: start.x,
            offset: 0.0,
            step: 0.0,
            nav,
        };
        Self::drag(touch);
    }

    fn drag(touch: &Touch) {
        let Phase::Dragging {
            start, offset, nav, ..
        } = *PHASE.get_mut()
        else {
            return;
        };
        if !nav.is_ok() {
            Self::reset_swipe();
            return;
        }
        let width = nav.width();
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
        nav.place_pages(now, width);

        if touch.is_ended() {
            Self::release(nav);
        }
    }

    fn release(nav: Weak<Self>) {
        let Phase::Dragging { offset, step, .. } = take(PHASE.get_mut()) else {
            return;
        };
        let width = nav.width();
        let finish = offset > width / 2.0 || step >= FLING;
        let target = if finish { width } else { 0.0 };
        log::debug!("swipe back released at {offset} of {width}, step {step}, finish {finish}");

        *PHASE.get_mut() = Phase::Settling;
        let distance = (target - offset).abs();
        if distance < 0.5 || width <= 0.0 {
            nav.complete(finish);
            return;
        }

        let lock: TouchLock = Touch::lock();
        let animation = UIAnimation::new(move |_, x| nav.place_pages(x, width)).animation(Animation::new(
            offset,
            target,
            SLIDE * distance / width,
        ));
        let mut lock = Some(lock);
        animation.on_finish.sub(move || {
            drop(lock.take());
            nav.complete(finish);
        });
        nav.top().add_animation(animation);
    }

    fn complete(self: Weak<Self>, finish: bool) {
        // A screen set while the slide ran has reset the swipe already.
        if !matches!(*PHASE.get_mut(), Phase::Settling) {
            return;
        }
        *PHASE.get_mut() = Phase::Idle;
        let below = self.below();
        let top = self.top();
        if finish {
            log::info!("swipe back left {}", top.page().label());
            self.drop_top(top);
            return;
        }
        top.set_shift(0.0);
        top.set_color(CLEAR);
        below.set_shift(0.0);
        below.set_covered(false);
        below.show(false);
    }

    /// The top page `offset` points right of its place, the page below a
    /// share of the rest left of its own.
    fn place_pages(&self, offset: f32, width: f32) {
        let behind = if width > 0.0 {
            -width * PARALLAX * (1.0 - offset / width)
        } else {
            0.0
        };
        self.top().set_shift(offset);
        self.below().set_shift(behind);
    }
}
