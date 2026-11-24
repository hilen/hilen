use std::mem::take;

use log::debug;
use ui_proc::view;

use super::motion::{Speed, Spring, rubber, target};
use crate::{
    deps::{
        hreads::after,
        refs::{Own, Weak},
        vents::Event,
    },
    gm::{Animation, LossyConvert, flat::Point},
    time::frame_seconds,
    ui::{
        Setup, SideSwipe, SideSwipeEvent, TouchStack, UIAnimation, UIEvents, UIManager, View, ViewData,
        ViewFrame, ViewSubviews, WeakView,
    },
};

/// A sideways wheel or trackpad scroll that rests this long ends, then the
/// pages go into place.
const WHEEL_REST: f32 = 0.12;

/// The pages of a `PageView`, each by a key, like the data source of a
/// `UIPageViewController`. A key stays right when the list changes while a
/// finger turns a page.
pub trait PageSource {
    /// The key of the page after `key`, or before it when `forward` is false.
    /// `None` at the end, the pages then pull back like a rubber band.
    fn page_beside(&self, key: &str, forward: bool) -> Option<String>;
    /// A new view for the page of `key`. The page view owns it and drops it
    /// when the page is no longer open or beside the open one.
    fn make_page(&mut self, key: &str) -> Own<dyn View>;

    /// The view of `key` is on the page view now and has run its `setup`,
    /// so it takes its data here.
    fn page_made(&mut self, key: &str, view: WeakView) {
        debug!("page view added the page {key}, {}", view.label());
    }

    /// The view of `key` goes, the page view removes it right after this.
    /// The last moment to keep what the page holds, like a draft.
    fn page_dropped(&mut self, key: &str, view: WeakView) {
        debug!("page view drops the page {key}, {}", view.label());
    }
}

/// A turn of the pages that ended, like `didFinishAnimating` of `UIKit`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageTurn {
    /// The page that is open now.
    pub key:       String,
    /// The page beside became the open one. False when the pages went back.
    pub completed: bool,
}

/// A page with its place, -1 before the open one, 0 open, 1 after it.
struct Page {
    key:  String,
    view: WeakView,
    slot: i8,
}

#[derive(Clone, Copy)]
struct Drag {
    /// Where the open page was when the finger took it.
    from:   f32,
    /// The finger offset at that moment, the pages follow from there with no
    /// jump.
    base:   f32,
    /// `PageView::anchor` at that moment.
    anchor: f32,
}

/// Pages that a finger turns sideways, like `UIPageViewController` with the
/// scroll style. The pages lie in 1 strip, and the page nearest the middle is
/// the open one at every moment, also while a finger drags or the pages
/// glide, so a swipe on gliding pages goes on to the page after the next one
/// at once. A page is a view, made by a `PageSource` when the pages first move
/// toward it, and the pages beside the open one are dropped once they rest.
/// Every move is measured on a real `UIPageViewController`, see
/// `docs/page-view.md`: the pages follow the finger, pull like a rubber band
/// past the last one, turn past half the width or with a flick of 300 points
/// a second, and go into place on the spring of `UIKit`. A finger on gliding
/// pages stops them where they are.
///
/// It turns by a finger where a finger drags the content, see
/// `UIManager::drag_scrolling`, and by a sideways wheel or trackpad scroll
/// everywhere.
#[view]
pub struct PageView {
    source:  Weak<dyn PageSource>,
    pages:   Vec<Page>,
    /// How far right of its place the open page is.
    shift:   f32,
    spacing: f32,
    drag:    Option<Drag>,
    speed:   Speed,
    /// Counts the glides, an older one stops moving the pages.
    run:     u32,
    gliding: bool,
    /// A finger came down on gliding pages and stopped them.
    held:    bool,
    /// The page `will_turn` told about last in this move.
    told:    Option<String>,
    /// The open page when this move of the pages began.
    from:    Option<String>,
    /// Grows by a step each time the open page moves a slot, so a drag and a
    /// glide keep their own frame while the slots change under them.
    anchor:  f32,
    /// The finger offset a wheel or trackpad scroll adds up to.
    wheel:   f32,
    wheels:  u32,

    /// A finger moved toward the page of this key, like
    /// `willTransitionTo` of `UIKit`.
    pub will_turn: Event<String>,
    /// The pages rest after a move that `will_turn` began.
    pub turned:    Event<PageTurn>,
}

impl Setup for PageView {
    fn setup(self: Weak<Self>) {
        SideSwipe::set_held(self, move |event| self.swiped(event), move |down| self.held(down));
        UIEvents::on_scroll().val(self, move |delta| self.wheeled(delta));
    }

    fn clips_to_bounds(&self) -> bool {
        true
    }
}

impl PageView {
    pub fn set_source(mut self: Weak<Self>, source: Weak<dyn PageSource>) -> Weak<Self> {
        self.source = source;
        self
    }

    /// The points between 2 pages while they move, `interPageSpacing` of
    /// `UIKit`. 0 by default.
    pub fn set_spacing(mut self: Weak<Self>, spacing: impl Into<f32>) -> Weak<Self> {
        self.spacing = spacing.into();
        self
    }

    /// How far right of its place the open page stands, in points, like the
    /// `contentOffset` of the scroll view of `UIKit` with the other sign. Less
    /// than 0 while the page after it comes in. A row of page dots can follow
    /// it.
    pub fn offset(&self) -> f32 {
        self.shift
    }

    /// The key of the open page.
    pub fn page(&self) -> Option<&str> {
        self.pages.iter().find(|page| page.slot == 0).map(|page| page.key.as_str())
    }

    /// Opens the page of `key` at once, with no move. The page that is open
    /// already stays as it is.
    pub fn set_page(mut self: Weak<Self>, key: impl ToString) -> Weak<Self> {
        let key = key.to_string();
        self.stop();
        self.shift = 0.0;
        if self.page() == Some(key.as_str()) {
            self.drop_pages(|page| page.slot != 0);
            return self;
        }
        self.drop_pages(|_| true);
        self.add_page(key, 0);
        self
    }

    /// The open page is known as `key` from now on, the view stays. For a
    /// page that took its key late, like a draft that became a saved item.
    pub fn rename_page(mut self: Weak<Self>, key: impl ToString) -> Weak<Self> {
        let key = key.to_string();
        if let Some(page) = self.pages.iter_mut().find(|page| page.slot == 0) {
            debug!("page view renames the page {} to {key}", page.key);
            page.key = key;
        }
        self.drop_pages(|page| page.slot != 0);
        self
    }

    /// No page is open, every page goes.
    pub fn clear(mut self: Weak<Self>) -> Weak<Self> {
        self.stop();
        self.shift = 0.0;
        self.drop_pages(|_| true);
        self
    }

    /// Opens the page of `key` with the move of a turn, from the right with
    /// `forward`, like `setViewControllers` with animation.
    pub fn turn_to(mut self: Weak<Self>, key: impl ToString, forward: bool) -> Weak<Self> {
        let key = key.to_string();
        if self.page().is_none() || self.page() == Some(key.as_str()) {
            return self.set_page(key);
        }
        self.stop();
        self.drop_pages(|page| page.slot != 0);
        self.from = self.page().map(str::to_string);
        let slot: i8 = if forward { 1 } else { -1 };
        self.add_page(key.clone(), slot);
        self.told = Some(key.clone());
        self.will_turn.trigger(key);
        self.glide(slot);
        self
    }

    /// The pages stop where they are, a glide that runs no longer moves them.
    fn stop(mut self: Weak<Self>) {
        self.run += 1;
        self.gliding = false;
        self.held = false;
        self.drag = None;
    }

    fn step(&self) -> f32 {
        self.width() + self.spacing
    }

    fn has(&self, slot: i8) -> bool {
        self.pages.iter().any(|page| page.slot == slot)
    }

    fn add_page(mut self: Weak<Self>, key: String, slot: i8) {
        if !self.source.is_ok() {
            debug!("page view has no source for the page {key}");
            return;
        }
        let view = self.source.make_page(&key);
        let view = self.add_subview(view).weak_view();
        self.place_page(view, slot);
        self.pages.push(Page {
            key: key.clone(),
            view,
            slot,
        });
        self.source.page_made(&key, view);
    }

    fn place_page(self: Weak<Self>, view: WeakView, slot: i8) {
        let slot = f32::from(slot);
        view.place().clear().back().custom(move |frame| {
            if self.is_ok() {
                frame.origin.x += self.shift + slot * (frame.size.width + self.spacing);
            }
        });
    }

    fn drop_pages(mut self: Weak<Self>, drop: impl Fn(&Page) -> bool) {
        let (gone, kept): (Vec<Page>, Vec<Page>) =
            take(&mut self.pages).into_iter().partition(|page| drop(page));
        self.pages = kept;
        for mut page in gone {
            if self.source.is_ok() {
                self.source.page_dropped(&page.key, page.view);
            }
            if page.view.is_ok() {
                page.view.remove_from_superview();
            }
        }
    }

    /// Makes the page beside the open one at `slot`, once.
    fn peek(mut self: Weak<Self>, slot: i8) {
        if self.has(slot) || !self.source.is_ok() {
            return;
        }
        let Some(open) = self.page().map(str::to_string) else {
            return;
        };
        let Some(key) = self.source.page_beside(&open, slot > 0) else {
            return;
        };
        debug!("page view makes the page {key} beside {open}");
        self.add_page(key.clone(), slot);
        self.told = Some(key.clone());
        self.will_turn.trigger(key);
    }

    /// The page at `toward`, 1 after the open one or -1 before it, is the
    /// open one now. The pages stay where they are on screen.
    fn recenter(mut self: Weak<Self>, toward: i8) {
        let step = self.step() * f32::from(toward);
        self.shift += step;
        self.anchor += step;
        for page in &mut self.pages {
            page.slot -= toward;
        }
        let placed: Vec<(WeakView, i8)> = self.pages.iter().map(|page| (page.view, page.slot)).collect();
        for (view, slot) in placed {
            self.place_page(view, slot);
        }
        debug!("page view passed half a page, {:?} is open", self.page());
    }

    /// The open page follows the pages past half a step, and the page beside
    /// it in the way they move is made.
    fn settle_slots(self: Weak<Self>) {
        let half = self.step() / 2.0;
        loop {
            if self.shift < 0.0 {
                self.peek(1);
            } else if self.shift > 0.0 {
                self.peek(-1);
            }
            if self.shift < -half && self.has(1) {
                self.recenter(1);
            } else if self.shift > half && self.has(-1) {
                self.recenter(-1);
            } else {
                return;
            }
        }
    }

    fn swiped(self: Weak<Self>, event: SideSwipeEvent) {
        if !self.is_ok() || self.page().is_none() {
            return;
        }
        match event {
            SideSwipeEvent::Moved(offset) => {
                if self.drag.is_none() {
                    self.begin(offset);
                }
                self.follow(offset, frame_seconds());
            }
            SideSwipeEvent::Ended(_) => self.release(frame_seconds()),
        }
    }

    /// A finger came down on the pages, or went away with no swipe.
    fn held(mut self: Weak<Self>, down: bool) {
        if !self.is_ok() {
            return;
        }
        if down {
            if self.gliding {
                debug!("a finger stopped the pages at {}", self.shift);
                self.stop();
                self.held = true;
            }
        } else if self.held {
            self.held = false;
            let page = target(self.shift, 0.0, self.step(), self.has(-1), self.has(1));
            self.glide(page);
        }
    }

    fn begin(mut self: Weak<Self>, base: f32) {
        let from = self.shift;
        let resting = !self.gliding && !self.held;
        self.stop();
        if resting {
            self.told = None;
            self.from = self.page().map(str::to_string);
        }
        let anchor = self.anchor;
        self.drag = Some(Drag { from, base, anchor });
        self.speed.clear();
    }

    fn follow(mut self: Weak<Self>, offset: f32, seconds: f64) {
        let Some(drag) = self.drag else {
            return;
        };
        self.speed.add(seconds, offset);
        self.shift = drag.from + offset - drag.base + self.anchor - drag.anchor;
        self.settle_slots();
        let to = drag.from + offset - drag.base + self.anchor - drag.anchor;
        let step = self.step();
        let width = self.width();
        let low = if self.has(1) { -step } else { 0.0 };
        let high = if self.has(-1) { step } else { 0.0 };
        self.shift = if to > high {
            high + rubber(to - high, width)
        } else if to < low {
            low - rubber(low - to, width)
        } else {
            to
        };
    }

    fn release(mut self: Weak<Self>, seconds: f64) {
        if self.drag.take().is_none() {
            return;
        }
        let speed = self.speed.at_release(seconds);
        let page = target(self.shift, speed, self.step(), self.has(-1), self.has(1));
        debug!(
            "page view released at {} with {speed} points a second, goes to {page}",
            self.shift
        );
        self.glide(page);
    }

    /// The pages go to the page at `slot` beside the open one on the spring
    /// of `UIKit`.
    fn glide(mut self: Weak<Self>, slot: i8) {
        self.run += 1;
        let run = self.run;
        let to = -f32::from(slot) * self.step();
        let spring = Spring::new(self.shift, to);
        let seconds = spring.seconds();
        if seconds <= 0.0 {
            self.shift = to;
            self.settle_slots();
            self.finish();
            return;
        }
        self.gliding = true;
        let anchor = self.anchor;
        let page = self;
        let length: f32 = seconds.lossy_convert();
        let animation = UIAnimation::new(move |_, part| {
            if page.is_ok() && page.run == run {
                page.set_shift(spring.at(f64::from(part) * seconds) + page.anchor - anchor);
            }
        })
        .animation(Animation::new(0.0, 1.0, length));
        animation.on_finish.sub(move || {
            if page.is_ok() && page.run == run {
                page.set_shift(to + page.anchor - anchor);
                page.finish();
            }
        });
        self.add_animation(animation);
    }

    fn set_shift(mut self: Weak<Self>, shift: f32) {
        self.shift = shift;
        self.settle_slots();
    }

    /// The pages rest on the open page, the pages beside it go.
    fn finish(mut self: Weak<Self>) {
        self.gliding = false;
        self.shift = 0.0;
        self.drop_pages(|page| page.slot != 0);
        let Some(key) = self.page().map(str::to_string) else {
            return;
        };
        let from = self.from.take();
        if self.told.take().is_some() {
            let completed = from.as_deref() != Some(key.as_str());
            debug!("page view turned to {key}, completed {completed}");
            self.turned.trigger(PageTurn { key, completed });
        }
    }

    /// A sideways wheel or trackpad scroll moves the pages like a finger,
    /// and they go into place once it rests.
    fn wheeled(mut self: Weak<Self>, delta: Point) {
        if !self.is_ok()
            || delta.x.abs() <= delta.y.abs()
            || self.page().is_none()
            || self.is_hidden_in_tree()
            || TouchStack::overlay_count() > 0
        {
            return;
        }
        let cursor = UIManager::cursor_position();
        if !self.contains_visible(cursor) {
            return;
        }
        // A view under the cursor that scrolls sideways by itself takes it.
        let pans = TouchStack::scrolls().any(|scroll| {
            scroll.is_ok()
                && scroll.__moves_sideways()
                && !scroll.is_hidden_in_tree()
                && scroll.contains_visible(cursor)
        });
        if pans {
            return;
        }
        if self.drag.is_none() {
            self.wheel = 0.0;
            self.begin(0.0);
        }
        self.wheel += delta.x;
        let seconds = frame_seconds();
        self.follow(self.wheel, seconds);
        self.wheels += 1;
        let wheels = self.wheels;
        after(WHEEL_REST, move || {
            if self.is_ok() && self.wheels == wheels {
                self.release(seconds);
            }
        });
    }
}
