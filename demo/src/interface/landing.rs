use hilen::{
    level::LevelManager,
    refs::Weak,
    ui::{Anchor, Label, Point, Setup, Slider, ViewData, ViewFrame, view},
};

use crate::{
    interface::palette::TEXT_DIM,
    levels::{ChamberLevel, chamber_level::RADIUS},
};

const MAX_SPEED: f32 = 3.0;
// The strip under the chamber that holds the speed slider.
const CONTROLS_HEIGHT: f32 = 72.0;

// The share of the view the chamber takes.
const FILL: f32 = 0.85;

/// The first screen, the chamber level under an empty view. The level
/// draws under the whole window, so the camera keeps the chamber
/// centered in this view, not in the window. A slider under the
/// chamber sets how fast the blades turn.
#[view]
pub struct Landing {
    #[init]
    speed:       Slider,
    speed_label: Label,
}

impl Setup for Landing {
    fn setup(mut self: Weak<Self>) {
        LevelManager::set_level(ChamberLevel::default());

        self.speed.set_horizontal().place().b(20).center_x().size(220, 32);
        self.speed.set_range(0, MAX_SPEED).set_value(1);
        self.speed.on_change.val(move |speed| {
            LevelManager::downcast_level::<ChamberLevel>().speed = speed;
            self.speed_label.set_text(format!("{speed:.1}x"));
        });

        self.speed_label
            .set_text("1.0x")
            .set_text_color(TEXT_DIM)
            .set_text_size(14)
            .place()
            .size(60, 24)
            .same_y(self.speed)
            .anchor(Anchor::Left, self.speed, 12);

        self.size_changed().sub(move || self.aim_camera());
        self.aim_camera();
    }
}

impl Landing {
    /// Scale the level so the chamber fills the view, then aim the
    /// camera. The camera position is the level point at the window
    /// center and the chamber sits at the level origin, so the camera
    /// moves by the offset from the window center to the view center.
    fn aim_camera(self: Weak<Self>) {
        let mut frame = *self.absolute_frame();
        frame.size.height -= CONTROLS_HEIGHT;
        let root = hilen::ui::UIManager::root_view().frame().size;
        let side = frame.width().min(frame.height()) * FILL;
        LevelManager::set_points_per_unit(side / (2.0 * (RADIUS + 1.0)));

        let center_x = frame.x() + frame.width() / 2.0;
        let center_y = frame.y() + frame.height() / 2.0;
        let dx = center_x - root.width / 2.0;
        let dy = center_y - root.height / 2.0;
        let ppu = LevelManager::points_per_unit();
        let camera = Point::new(-dx / ppu, dy / ppu);
        *LevelManager::camera_pos() = camera;

        // Objects start above the window top so they are never seen
        // popping into being.
        let window_top = camera.y + root.height / 2.0 / ppu;
        LevelManager::downcast_level::<ChamberLevel>().spawn_y = window_top + 4.0;
    }
}
