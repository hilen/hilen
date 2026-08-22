use hilen::{
    level::{Control, LevelManager},
    refs::Weak,
    ui::{
        BLACK, BlurView, CornerRadii, DPadView, Label, Setup, StickView, TextAlignment, ViewData,
        ViewSubviews, WHITE, view,
    },
};

use crate::{interface::scenes::add_back_button, levels::TestLevel};

/// Shows the signature blur-over-game effect. The physics level runs as
/// the backdrop and a frosted glass bar floats on top, so the game is
/// blurred inside the bar while the controls on it stay crisp.
#[view]
pub struct FrostedHud {
    level: Weak<TestLevel>,

    #[init]
    panel: BlurView,
}

impl Setup for FrostedHud {
    fn setup(mut self: Weak<Self>) {
        self.level = LevelManager::set_level(TestLevel::default());

        self.panel.set_blur_radius(12).set_color(BLACK.with_alpha(0.35));
        self.panel.set_corner_radii(CornerRadii::top(24));
        self.panel.place().b(0).lr(0).h(150);

        let title = self.panel.add_view::<Label>();
        title
            .set_text("Frosted glass over the game")
            .set_text_color(WHITE)
            .set_text_size(16)
            .set_alignment(TextAlignment::Center);
        title.place().t(10).lr(10).h(22);

        let dpad = self.panel.add_view::<DPadView>();
        dpad.place().bl(16).size(110, 80);
        dpad.on_press.val(move |direction| {
            self.level.player.unit.body.move_by_direction(direction);
        });

        let stick = self.panel.add_view::<StickView>();
        stick.place().br(16).size(110, 110);
        stick.on_change.val(move |direction| {
            self.level.player.unit.body.add_impulse(direction.invert_y() / 500.0);
        });

        add_back_button(self);
    }
}
