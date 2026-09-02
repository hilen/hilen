use hilen::{
    refs::Weak,
    scene::SceneManager,
    ui::{Button, Point, Setup, UIManager, ViewData, ViewTouch, WHITE, view},
};

use crate::{
    interface::{
        HomeView,
        palette::{ACCENT, BORDER, SURFACE, TEXT},
    },
    scenes::DemoScene,
};

/// Radians of look per point of drag.
const LOOK_SPEED: f32 = 0.004;

/// The 3D playground page, seen through the player's eyes. Drag to
/// look around, `w` `a` `s` `d` or the arrows walk, space jumps, drop
/// balls onto the pyramid.
#[view]
pub struct Scene3D {
    scene:      Weak<DemoScene>,
    last_touch: Point,

    #[init]
    back: Button,
    drop: Button,
}

impl Setup for Scene3D {
    fn setup(mut self: Weak<Self>) {
        self.scene = SceneManager::set_scene(DemoScene::default());

        self.enable_touch();
        self.touch().began.val(move |touch| {
            self.last_touch = touch.position;
        });
        self.touch().moved.val(move |touch| {
            let dx = touch.position.x - self.last_touch.x;
            let dy = touch.position.y - self.last_touch.y;
            self.last_touch = touch.position;
            if let Some(player) = self.scene.player.as_mut() {
                player.look(dx * LOOK_SPEED, -dy * LOOK_SPEED);
            }
        });

        self.back
            .set_color(ACCENT)
            .set_text_color(WHITE)
            .set_corner_radius(10)
            .set_text("Back");
        self.back.place().tl(20).size(90, 40);
        self.back.on_tap(|| {
            SceneManager::stop_scene();
            UIManager::set_view(HomeView::new());
        });

        self.drop
            .set_text("Drop ball")
            .set_color(SURFACE)
            .set_text_color(TEXT)
            .set_corner_radius(10)
            .set_border_width(1)
            .set_border_color(BORDER);
        self.drop.place().t(20).r(20).size(120, 40);
        self.drop.on_tap(move || self.scene.drop_ball());
    }
}
