use hilen::{
    level::LevelManager,
    refs::Weak,
    ui::{Point, Setup, ViewData, ViewFrame, view},
};

use crate::levels::{ChamberLevel, chamber_level::RADIUS};

// The share of the view the chamber takes.
const FILL: f32 = 0.85;

/// The first screen, the chamber level under an empty view. The level
/// draws under the whole window, so the camera keeps the chamber
/// centered in this view, not in the window.
#[view]
pub struct Landing {}

impl Setup for Landing {
    fn setup(self: Weak<Self>) {
        LevelManager::set_level(ChamberLevel::default());

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
        let frame = self.absolute_frame();
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
