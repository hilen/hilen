use hilen::{
    level::LevelManager,
    refs::Weak,
    ui::{Button, Setup, ViewData, view},
};

use crate::{interface::HomeView, no_physics::NoPhysicsLevel};

#[view]
pub struct NoPhysicsView {
    #[init]
    back: Button,
}

impl Setup for NoPhysicsView {
    fn setup(self: Weak<Self>) {
        LevelManager::set_level(NoPhysicsLevel::default());

        self.back.set_text("Back");
        self.back.place().size(100, 50).t(100).l(20);
        self.back.add_transition::<Self, HomeView>();
    }
}
