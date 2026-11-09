#[cfg(feature = "gpu")]
use std::ops::DerefMut;

use educe::Educe;

use crate::{
    deps::{netrun::Function, refs::Weak, vents::OnceEvent},
    gm::Animation,
    ui::{View, WeakView},
};

#[cfg(feature = "gpu")]
type Action = Box<dyn FnMut(&mut dyn View, f32) + Send>;

#[derive(Educe)]
#[educe(Debug)]
pub struct UIAnimation {
    #[educe(Debug(ignore))]
    pub(crate) view: WeakView,
    animation:       Animation,
    #[educe(Debug(ignore))]
    #[cfg(feature = "gpu")]
    action:          Action,
    #[educe(Debug(ignore))]
    pub on_finish:   OnceEvent,

    finish_condition:   Function<(), bool>,
    /// Made by `repeat`, a closure animation with no end.
    pub(crate) repeats: bool,
}

impl UIAnimation {
    pub fn new(action: impl FnMut(&mut dyn View, f32) + Send + 'static) -> Self {
        // A build with no window draws no frame, nothing would run the action.
        #[cfg(not(feature = "gpu"))]
        drop(action);
        Self {
            view:                           Weak::default(),
            animation:                      Animation::default(),
            #[cfg(feature = "gpu")]
            action:                         Box::new(action),
            on_finish:                      OnceEvent::default(),
            finish_condition:               Function::default(),
            repeats:                        false,
        }
    }

    pub fn animation(mut self, animation: Animation) -> Self {
        self.animation = animation;
        self
    }

    /// Never finishes. The value keeps bouncing between the ends until the
    /// view is gone. A thing that repeats is better a declared loop,
    /// `ViewData::run_loop`, which the system can run with no frame of
    /// the app. This one writes a warning once per view.
    pub fn repeat(mut self) -> Self {
        self.repeats = true;
        self.finish_condition(|| false)
    }

    pub(crate) fn finish_condition(self, mut finish: impl FnMut() -> bool + Send + 'static) -> Self {
        self.finish_condition.replace(move |()| finish());
        self
    }

    #[cfg(feature = "gpu")]
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

    #[cfg(feature = "gpu")]
    pub(crate) fn commit(&mut self) {
        let value = if self.animation.is_empty() {
            0.0
        } else {
            self.animation.value()
        };
        (self.action)(self.view.deref_mut(), value);
    }
}
