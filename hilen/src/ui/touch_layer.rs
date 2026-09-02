use crate::{
    deps::refs::{
        Weak,
        vec::{WeakVec, WeakVecHelper},
    },
    gm::flat::Point,
    ui::{Touch, View, ViewData, WeakView, view::ViewSubviews},
};

pub(crate) trait Scrollable: View {
    fn __process_scroll_touch(&mut self, touch: Touch) -> bool;
    fn __process_wheel_scroll(&mut self, delta: Point);
}

pub(crate) struct TouchLayer {
    pub(crate) root: WeakView,
    listeners:       Vec<WeakView>,
    /// Checked before every plain listener, whenever registered. A split
    /// divider's grab zone overlaps panel content that registers later,
    /// and registration order alone would hand the divider's touches to
    /// that content.
    high_priority:   Vec<WeakView>,
    hovered:         Vec<WeakView>,
    high_hovered:    Vec<WeakView>,
    scrolls:         WeakVec<dyn Scrollable>,
}

/// Registrations pulled out of one layer to move into another when a
/// layer is pushed over existing views or popped from under them.
#[derive(Default)]
pub(crate) struct Extracted {
    listeners:     Vec<WeakView>,
    high_priority: Vec<WeakView>,
    hovered:       Vec<WeakView>,
    high_hovered:  Vec<WeakView>,
    scrolls:       WeakVec<dyn Scrollable>,
}

impl TouchLayer {
    pub(crate) fn add_scroll(&mut self, view: Weak<dyn Scrollable>) {
        if self.scrolls.iter().any(|l| l.raw() == view.raw()) {
            return;
        }
        self.scrolls.push(view);
    }

    pub fn add(&mut self, view: WeakView) {
        if self.listeners.iter().any(|l| l.raw() == view.raw()) {
            return;
        }
        self.listeners.push(view);
    }

    pub(crate) fn add_low_priority(&mut self, view: WeakView) {
        if self.listeners.iter().any(|l| l.raw() == view.raw()) {
            return;
        }
        self.listeners.insert(0, view);
    }

    pub(crate) fn add_high_priority(&mut self, view: WeakView) {
        if self.high_priority.iter().any(|l| l.raw() == view.raw()) {
            return;
        }
        self.high_priority.push(view);
    }

    pub(crate) fn add_hover(&mut self, view: WeakView) {
        if self.hovered.iter().any(|l| l.raw() == view.raw()) {
            return;
        }
        self.hovered.push(view);
    }

    pub(crate) fn add_hover_high_priority(&mut self, view: WeakView) {
        if self.high_hovered.iter().any(|l| l.raw() == view.raw()) {
            return;
        }
        self.high_hovered.push(view);
    }

    pub(crate) fn remove(&mut self, view: WeakView) {
        self.listeners.retain(|a| a.raw() != view.raw());
        self.high_priority.retain(|a| a.raw() != view.raw());
    }

    /// Moves every registration sitting under `root` out of this layer,
    /// so a pre-built overlay keeps its views when it becomes a layer.
    pub(crate) fn extract_under(&mut self, root: WeakView) -> Extracted {
        let root_raw = root.raw();
        let is_under = |view: WeakView| {
            let mut cur = view;
            while cur.is_ok() {
                if cur.raw() == root_raw {
                    return true;
                }
                cur = *cur.superview();
            }
            false
        };
        let mut extracted = Extracted::default();
        self.listeners.retain(|v| {
            if is_under(*v) {
                extracted.listeners.push(*v);
                false
            } else {
                true
            }
        });
        self.high_priority.retain(|v| {
            if is_under(*v) {
                extracted.high_priority.push(*v);
                false
            } else {
                true
            }
        });
        self.hovered.retain(|v| {
            if is_under(*v) {
                extracted.hovered.push(*v);
                false
            } else {
                true
            }
        });
        self.high_hovered.retain(|v| {
            if is_under(*v) {
                extracted.high_hovered.push(*v);
                false
            } else {
                true
            }
        });
        self.scrolls.retain(|s| {
            if s.is_ok() && is_under(s.weak_view()) {
                extracted.scrolls.push(*s);
                false
            } else {
                true
            }
        });
        extracted
    }

    /// The reverse of `extract_under`: a popped overlay layer hands its
    /// surviving registrations back, so reopening the overlay finds its
    /// buttons and scrolls still alive.
    pub(crate) fn absorb(&mut self, extracted: Extracted) {
        for view in extracted.listeners {
            if view.is_ok() {
                self.add(view);
            }
        }
        for view in extracted.high_priority {
            if view.is_ok() {
                self.add_high_priority(view);
            }
        }
        for view in extracted.hovered {
            if view.is_ok() {
                self.add_hover(view);
            }
        }
        for view in extracted.high_hovered {
            if view.is_ok() {
                self.add_hover_high_priority(view);
            }
        }
        for scroll in extracted.scrolls {
            if scroll.is_ok() {
                self.add_scroll(scroll);
            }
        }
    }

    pub(crate) fn drain(&mut self) -> Extracted {
        Extracted {
            listeners:     std::mem::take(&mut self.listeners),
            high_priority: std::mem::take(&mut self.high_priority),
            hovered:       std::mem::take(&mut self.hovered),
            high_hovered:  std::mem::take(&mut self.high_hovered),
            scrolls:       std::mem::take(&mut self.scrolls),
        }
    }

    /// Plain listeners first, the high priority tier after them. Dispatch
    /// iterates the result reversed, so the tier is checked first.
    pub(crate) fn views(&self) -> Vec<WeakView> {
        let mut views = self.listeners.clone();
        views.extend_from_slice(&self.high_priority);
        views
    }

    pub(crate) fn hovered(&self) -> Vec<WeakView> {
        let mut views = self.hovered.clone();
        views.extend_from_slice(&self.high_hovered);
        views
    }

    pub(crate) fn scrolls(&self) -> WeakVec<dyn Scrollable> {
        self.scrolls.clone()
    }

    pub(crate) fn root_name(&self) -> &str {
        self.root.label()
    }

    pub(crate) fn clear_freed(&mut self) {
        assert!(self.root.is_ok());
        self.listeners.remove_freed();
        self.high_priority.remove_freed();
        self.hovered.remove_freed();
        self.high_hovered.remove_freed();
        self.scrolls.remove_freed();
    }
}

impl From<WeakView> for TouchLayer {
    fn from(root: WeakView) -> Self {
        Self {
            root,
            listeners: vec![],
            high_priority: vec![],
            hovered: vec![],
            high_hovered: vec![],
            scrolls: vec![],
        }
    }
}
