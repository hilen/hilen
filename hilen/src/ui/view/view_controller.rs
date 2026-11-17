use std::sync::mpsc::{Receiver, channel};

use crate::{
    deps::{hreads::on_main, refs::Own},
    gm::Animation,
    ui::{
        BackButton, NavigationView, Touch, UIAnimation, UIManager, View, ViewData, ViewFrame, ViewSubviews,
    },
};

pub trait ViewController {
    fn navigation(&self) -> Weak<NavigationView>;
    /// The way back of the pushed page this view is in, made by its
    /// navigation stack. The page places it and sets its look, a tap on it
    /// pops the page like the swipe from the left edge does. The first page
    /// of a stack has none.
    fn back_button(&self) -> Weak<BackButton>;
    /// Runs `action` when the pushed page this view is in leaves its stack,
    /// by the swipe, the back button, Escape or the Android back. The page
    /// is still alive while it runs. A swipe that slides back runs nothing.
    /// The first page of a stack never leaves, it panics there.
    fn on_pop(&self, action: impl FnMut() + Send + 'static);
    /// Runs `action` each time the page this view is in is on top again,
    /// after the page over it left the stack.
    fn on_return(&self, action: impl FnMut() + Send + 'static);
    fn present(self: Weak<Self>, view: Own<dyn View>) -> Receiver<()>;
}

// use dispatch::on_main;
use crate::deps::refs::Weak;
// use tokio::sync::oneshot::{Receiver, channel};

pub const PRESENT_ANIMATION_DURATION: f32 = 0.4;

impl<T: ?Sized + View + 'static> ViewController for T {
    fn navigation(&self) -> Weak<NavigationView> {
        assert!(
            self.superview().is_ok(),
            "Current view is not a part of a navigation stack"
        );

        if self.navigation_view().is_ok() {
            self.navigation_view()
        } else {
            self.superview().navigation()
        }
    }

    fn back_button(&self) -> Weak<BackButton> {
        self.navigation().back_button_of(self.weak_view())
    }

    fn on_pop(&self, action: impl FnMut() + Send + 'static) {
        self.navigation().on_pop_of(self.weak_view(), Box::new(action));
    }

    fn on_return(&self, action: impl FnMut() + Send + 'static) {
        self.navigation().on_return_of(self.weak_view(), Box::new(action));
    }

    /// Present new view replacing and deallocating current one
    fn present(mut self: Weak<Self>, view: Own<dyn View>) -> Receiver<()> {
        let touch_lock = Touch::lock();

        let (se, rc) = channel();

        on_main(move || {
            let view = UIManager::root_view().add_subview_to_root(view);
            view.set_frame(self.frame().with_zero_origin());
            let anim = UIAnimation::new(|view, y| {
                view.set_y(y);
            })
            .animation(Animation::new(self.height(), 0.0, PRESENT_ANIMATION_DURATION));

            anim.on_finish.sub(move || {
                self.remove_from_superview();
                view.place().back();
                drop(touch_lock);
                _ = se.send(());
            });

            view.add_animation(anim);
        });

        rc
    }
}
