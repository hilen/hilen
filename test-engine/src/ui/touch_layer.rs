use refs::{
    Weak,
    vec::{WeakVec, WeakVecHelper},
};

use crate::{
    gm::flat::Point,
    ui::{Touch, View, ViewData, WeakView},
};

pub(crate) trait Scrollable: View {
    fn __process_scroll_touch(&mut self, touch: Touch) -> bool;
    fn __process_wheel_scroll(&mut self, delta: Point);
}

pub(crate) struct TouchLayer {
    pub(crate) root: WeakView,
    listeners:       Vec<WeakView>,
    hovered:         Vec<WeakView>,
    scrolls:         WeakVec<dyn Scrollable>,
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

    pub(crate) fn add_hover(&mut self, view: WeakView) {
        if self.hovered.iter().any(|l| l.raw() == view.raw()) {
            return;
        }
        self.hovered.push(view);
    }

    pub(crate) fn remove(&mut self, view: WeakView) {
        self.listeners.retain(|a| a.raw() != view.raw());
    }

    pub(crate) fn views(&self) -> Vec<WeakView> {
        self.listeners.clone()
    }

    #[cfg(any(desktop, wasm))]
    pub(crate) fn hovered(&self) -> Vec<WeakView> {
        self.hovered.clone()
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
        self.hovered.remove_freed();
        self.scrolls.remove_freed();
    }
}

impl From<WeakView> for TouchLayer {
    fn from(root: WeakView) -> Self {
        Self {
            root,
            listeners: vec![],
            hovered: vec![],
            scrolls: vec![],
        }
    }
}
