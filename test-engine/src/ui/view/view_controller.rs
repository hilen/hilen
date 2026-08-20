use std::sync::mpsc::{Receiver, channel};

use hreads::on_main;
use refs::Own;

use crate::{
    gm::Animation,
    ui::{NavigationView, Touch, UIAnimation, UIManager, View, ViewData, ViewFrame, ViewSubviews},
};

pub trait ViewController {
    fn navigation(&self) -> Weak<NavigationView>;
    fn present(self: Weak<Self>, view: Own<dyn View>) -> Receiver<()>;
}

// use dispatch::on_main;
use refs::Weak;
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
