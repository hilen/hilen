use ui_proc::view;

use crate::{
    deps::{
        hreads::assert_main_thread,
        refs::{Own, Weak},
    },
    gm::Animation,
    ui::{
        Setup, Touch, TouchStack, UIAnimation, View, ViewData, WeakView,
        view::{ViewFrame, ViewSubviews},
    },
};

#[view]
pub struct NavigationView {
    first_view: Option<Own<dyn View>>,
}

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
        let mut view = self.first_view.take().unwrap();
        view.set_navigation_view(self);
        let view = self.add_subview(view);
        view.place().back();
    }
}

impl NavigationView {
    pub fn push(self: Weak<Self>, view: Own<dyn View>) {
        assert_main_thread();
        assert!(!self.subviews().is_empty(), "BUG: push from empty navigation");

        let touch_lock = Touch::lock();

        TouchStack::push_layer(view.weak_view());

        let prev_view = self.subviews().last().unwrap().weak_view();

        let mut view = self.add_subview(view);
        view.place().back();
        view.set_navigation_view(self);
        view.set_frame(self.frame().with_zero_origin());

        let anim = UIAnimation::new(|view, x| {
            view.set_x(x);
        })
        .animation(Animation::new(self.width(), 0.0, 0.5));

        anim.on_finish.sub(move || {
            drop(touch_lock);
            prev_view.set_hidden(true);
        });

        view.add_animation(anim);
    }

    pub fn pop(self: Weak<Self>) {
        assert_main_thread();
        assert!(self.subviews().len() > 1, "BUG: Nowhere to pop");

        let touch_lock = Touch::lock();

        let below = self.below_pop();
        below.set_hidden(false);
        let mut to_pop = self.subviews().last().unwrap().weak_view();

        let anim = UIAnimation::new(|view, x| {
            view.set_x(x);
        })
        .animation(Animation::new(0.0, self.width(), 0.5));

        anim.on_finish.sub(move || {
            to_pop.remove_from_superview();
            TouchStack::pop_layer(to_pop);
            drop(touch_lock);
        });

        to_pop.add_animation(anim);
    }

    fn below_pop(&self) -> WeakView {
        self.subviews()[self.subviews().len() - 2].weak_view()
    }
}
