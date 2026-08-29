use std::ops::DerefMut;

use educe::Educe;

use crate::{
    deps::{netrun::Function, refs::Weak, vents::OnceEvent},
    gm::Animation,
    ui::{View, WeakView},
};

type Action = Box<dyn FnMut(&mut dyn View, f32) + Send>;

#[derive(Educe)]
#[educe(Debug)]
pub struct UIAnimation {
    #[educe(Debug(ignore))]
    pub(crate) view: WeakView,
    animation:       Animation,
    #[educe(Debug(ignore))]
    action:          Action,
    #[educe(Debug(ignore))]
    pub on_finish:   OnceEvent,

    finish_condition: Function<(), bool>,
}

impl UIAnimation {
    pub fn new(action: impl FnMut(&mut dyn View, f32) + Send + 'static) -> Self {
        Self {
            view:             Weak::default(),
            animation:        Animation::default(),
            action:           Box::new(action),
            on_finish:        OnceEvent::default(),
            finish_condition: Function::default(),
        }
    }

    pub fn animation(mut self, animation: Animation) -> Self {
        self.animation = animation;
        self
    }

    /// Never finishes. The value keeps bouncing between the ends until the
    /// view is gone.
    pub fn repeat(self) -> Self {
        self.finish_condition(|| false)
    }

    pub(crate) fn finish_condition(self, mut finish: impl FnMut() -> bool + Send + 'static) -> Self {
        self.finish_condition.replace(move |()| finish());
        self
    }

    pub(crate) fn active(&self) -> bool {
        if self.view.is_null() {
            return false;
        }

        if self.finish_condition.is_empty() {
            self.animation.active()
        } else {
            !self.finish_condition.call(())
        }
    }

    pub(crate) fn commit(&mut self) {
        let value = if self.animation.is_empty() {
            0.0
        } else {
            self.animation.value()
        };
        (self.action)(self.view.deref_mut(), value);
    }
}
