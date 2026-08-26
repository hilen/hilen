use std::sync::OnceLock;

use nonempty::NonEmpty;
use parking_lot::{Mutex, MutexGuard};

use crate::{
    deps::{hreads::from_main, refs::Weak},
    ui::{
        LongPress, NO_TOUCH_ID, UIManager, View, WeakView,
        touch_layer::{Scrollable, TouchLayer},
        view::{ViewData, ViewSubviews},
    },
};

static STACK: OnceLock<Mutex<TouchStack>> = OnceLock::new();

pub struct TouchStack {
    stack: NonEmpty<TouchLayer>,
}

impl TouchStack {
    fn init() -> Mutex<Self> {
        Self {
            stack: NonEmpty::new(UIManager::get().root_view.weak_view().into()),
        }
        .into()
    }

    pub(crate) fn get() -> MutexGuard<'static, Self> {
        STACK.get_or_init(Self::init).lock()
    }
}

impl TouchStack {
    fn layer_for(&mut self, view: WeakView) -> &mut TouchLayer {
        for layer in self.stack.iter_mut().rev() {
            let root_raw = layer.root.raw();
            let mut cur: WeakView = view;
            while cur.is_ok() {
                if cur.raw() == root_raw {
                    return layer;
                }
                cur = *cur.superview();
            }
        }

        unreachable!("Failed to find touch layer for view: {}", view.label())
    }
}

impl TouchStack {
    pub(crate) fn touch_views() -> impl Iterator<Item = WeakView> {
        Self::get().stack.last().views().into_iter().rev()
    }

    pub(crate) fn hover_views() -> impl Iterator<Item = WeakView> {
        Self::get().stack.last().hovered().into_iter().rev()
    }

    pub(crate) fn scrolls() -> impl Iterator<Item = Weak<dyn Scrollable>> {
        Self::get().stack.last().scrolls().into_iter()
    }

    pub(crate) fn enable_scroll(scroll: Weak<dyn Scrollable>) {
        Self::get().layer_for(scroll).add_scroll(scroll);
    }

    pub(crate) fn enable_for(view: WeakView) {
        Self::get().layer_for(view).add(view);
    }

    pub(crate) fn enable_for_low_priority(view: WeakView) {
        Self::get().layer_for(view).add_low_priority(view);
    }

    pub(crate) fn enable_hover(view: WeakView) {
        Self::get().layer_for(view).add_hover(view);
    }

    pub(crate) fn disable_for(view: WeakView) {
        Self::get().layer_for(view).remove(view);
    }

    /// Moves every registration under `view` to the front of its layer,
    /// keeping their relative order, so a view drawn over its siblings,
    /// like a pinned sticky table row, also wins their touches.
    pub(crate) fn raise_subtree(view: WeakView) {
        let mut this = Self::get();
        let layer = this.layer_for(view);
        let extracted = layer.extract_under(view);
        layer.absorb(extracted);
    }

    /// Puts an overlay on top of the touch stack: only views under it
    /// receive touches until the matching `pop_layer`. Registrations
    /// already sitting under the new root migrate into the layer, so a
    /// pre-built overlay keeps its buttons and scrolls when it opens.
    pub fn push_layer(view: WeakView) {
        let mut this = Self::get();
        let mut layer: TouchLayer = view.into();
        for existing in this.stack.iter_mut() {
            layer.absorb(existing.extract_under(view));
        }
        this.stack.push(layer);
    }

    pub fn touch_root_name_for(view: WeakView) -> String {
        Self::get().layer_for(view).root_name().to_string()
    }

    /// Removes the top overlay layer. Its surviving registrations go
    /// back to the layer below, so an overlay that merely hides can
    /// reopen with its views still registered.
    pub fn pop_layer(view: WeakView) {
        let mut this = Self::get();
        let mut pop = this.stack.pop().unwrap();
        assert_eq!(
            pop.root.raw(),
            view.raw(),
            "Inconsistent pop_touch_view call. Expected: {} got: {}",
            pop.root_name(),
            view.label()
        );
        let remaining = pop.drain();
        this.stack.last_mut().absorb(remaining);
    }

    pub fn root_name() -> String {
        Self::get().stack.last().root_name().to_string()
    }

    /// A scroll drag claimed the touch: views that captured it on began
    /// must let it go so the release doesn't end as a tap.
    pub(crate) fn cancel_touch(id: usize) {
        LongPress::cancel(id);

        for view in Self::touch_views() {
            if view.is_ok() && view.__base_view().__touch_id == id {
                view.__base_view().__touch_id = NO_TOUCH_ID;
            }
        }
    }

    pub(crate) fn clear_freed(&mut self) {
        self.stack.tail.retain(|a| a.root.is_ok());

        for layer in self.stack.iter_mut() {
            layer.clear_freed();
        }
    }

    pub fn dump() -> Vec<Vec<String>> {
        from_main(|| {
            UIManager::free_deleted_views();
            TouchStack::get().clear_freed();

            let mut result = vec![];

            for layer in &Self::get().stack {
                let mut layer_vec = vec![];

                layer_vec.push(format!("Layer: {}", layer.root_name()));

                for view in layer.views() {
                    assert!(view.is_ok(), "Null view in touch stack");
                    layer_vec.push(view.label().to_string());
                }

                result.push(layer_vec);
            }

            result
        })
    }
}
