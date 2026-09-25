use std::ops::{Deref, DerefMut};

use educe::Educe;
use plat::Platform;
use rapier2d::{
    dynamics::{RigidBody, RigidBodyHandle},
    prelude::{Collider, ColliderHandle},
};

use crate::{
    deps::refs::{Own, Weak, main_lock::MainLock},
    gm::{
        Clock, LossyConvert, ToF32,
        flat::{Point, Rect},
    },
    level::{Level, level::LevelPhysics},
    ui::UIManager,
    window::Window,
};

static SELF: MainLock<LevelManager> = MainLock::new();

#[derive(Educe)]
#[educe(Default)]
pub struct LevelManager {
    #[educe(Default = 1.0)]
    scale:      f32,
    camera_pos: Point,

    #[educe(Default = Self::DEFAULT_STEP)]
    step:        f32,
    /// Clock time of the last frame, none until a level runs a frame.
    last_frame:  Option<f64>,
    /// Real time not yet simulated, less than one step after a frame.
    accumulator: f32,
    /// Simulated seconds since the level was set.
    time:        f64,

    level: Option<Own<dyn Level>>,

    scale_changed: Option<Box<dyn FnMut(f32)>>,
}

impl LevelManager {
    pub(crate) const fn default_z_position() -> f32 {
        0.85
    }

    pub(crate) const fn z_position_offset() -> f32 {
        0.000_001
    }

    /// The default fixed step. Twice the common 60 Hz frame, so a 120 Hz
    /// screen gets a new pose every frame too.
    pub const DEFAULT_STEP: f32 = 1.0 / 120.0;

    /// Steps one frame may run. A longer stall, a breakpoint or a window
    /// drag, drops the rest, the level slows down instead of running a
    /// burst of steps that makes the next frame late too.
    pub const MAX_STEPS_PER_FRAME: usize = 8;

    /// Runs the level for the real time since the last frame, in fixed
    /// steps, so it moves at the same speed at any frame rate. Real time
    /// is `Clock` time, a stepped test moves it frame by frame.
    pub(crate) fn update() {
        if Self::no_level() {
            return;
        }

        let cursor = Self::level_point(UIManager::cursor_position());
        unsafe { Self::level_unchecked() }.cursor_position = cursor;

        let now = Clock::now_ms();
        let state = SELF.get_mut();
        let elapsed: f32 = state
            .last_frame
            .map_or(0.0, |last| ((now - last).max(0.0) / 1000.0).lossy_convert());
        state.last_frame = Some(now);
        state.accumulator += elapsed;

        // The level's own update may reach the manager, so its state is
        // read again after every step.
        let mut steps = 0;
        while steps < Self::MAX_STEPS_PER_FRAME && !Self::no_level() {
            let state = SELF.get_mut();
            let step = state.step;
            if state.accumulator < step {
                return;
            }
            state.accumulator -= step;
            state.time += f64::from(step);
            steps += 1;
            Self::level().__internal_update(step);
        }
        SELF.get_mut().accumulator = 0.0;
    }
}

impl LevelManager {
    pub fn set_level<T: Level + 'static>(level: T) -> Weak<T> {
        let l = SELF.get_mut();
        let level = Own::new(level);
        let weak = level.weak();
        l.level = Some(level);
        l.last_frame = None;
        l.accumulator = 0.0;
        l.time = 0.0;
        l.level.as_ref().unwrap().__internal_setup();
        weak
    }

    pub fn stop_level() {
        SELF.get_mut().level = None;
        Self::set_scale(1.0);
        *Self::camera_pos() = (0, 0).into();
    }

    pub(crate) fn level() -> &'static dyn Level {
        SELF.level.as_ref().expect("No Level").deref()
    }

    pub fn level_weak() -> Weak<dyn Level> {
        SELF.level.as_ref().expect("No Level").weak()
    }

    pub(crate) unsafe fn level_unchecked() -> &'static mut dyn Level {
        unsafe { SELF.get_unchecked().level.as_mut().expect("No Level").deref_mut() }
    }

    pub(crate) fn physics() -> &'static mut LevelPhysics {
        unsafe {
            Self::level_unchecked()
                .physics
                .as_mut()
                .expect("This level has no physics enabled. Override LevelSetup::needs_physics to enable.")
        }
    }

    pub fn downcast_level<T: Level + 'static>() -> Weak<T> {
        Self::level_weak().downcast::<T>().unwrap()
    }

    pub(crate) fn get_rigid_body(handle: RigidBodyHandle) -> &'static RigidBody {
        &LevelManager::physics().sets.rigid_bodies[handle]
    }

    pub(crate) fn get_collider(handle: ColliderHandle) -> &'static Collider {
        &LevelManager::physics().sets.colliders[handle]
    }

    pub(crate) fn no_level() -> bool {
        SELF.level.is_none()
    }

    pub fn scale() -> f32 {
        SELF.get_mut().scale
    }

    pub fn set_scale(scale: f32) {
        let sf = SELF.get_mut();
        sf.scale = scale;
        if let Some(cb) = &mut sf.scale_changed {
            cb(scale);
        }
    }

    pub fn on_scale_changed(callb: impl FnMut(f32) + 'static) {
        let cb = &mut SELF.get_mut().scale_changed;
        assert!(cb.is_none());
        cb.replace(Box::new(callb));
    }

    /// The fixed step the level runs in, in seconds, `DEFAULT_STEP` unless
    /// set. `LevelSetup::update` gets it split by the physics substeps.
    pub fn step() -> f32 {
        SELF.step
    }

    pub fn set_step(seconds: impl ToF32) {
        let seconds = seconds.to_f32();
        assert!(seconds > 0.0, "a level step must be longer than zero");
        SELF.get_mut().step = seconds;
    }

    /// Seconds the running level has simulated since it was set.
    pub fn time() -> f64 {
        SELF.time
    }

    pub fn camera_pos() -> &'static mut Point {
        &mut SELF.get_mut().camera_pos
    }

    /// Screen points per level unit. The sprite shader draws ten pixels
    /// per unit times the level scale, and a point is a pixel over the
    /// screen scale, so an app can place a level point under a view.
    pub fn points_per_unit() -> f32 {
        10.0 * Self::scale() / Self::touch_screen_scale()
    }

    /// Pick the level scale that draws one level unit as `points` screen
    /// points, so a level can be fitted into a view of a known size.
    pub fn set_points_per_unit(points: f32) {
        Self::set_scale(points * Self::touch_screen_scale() / 10.0);
    }

    fn touch_screen_scale() -> f32 {
        if Platform::WINDOWS {
            Window::screen_scale().ceil()
        } else {
            Window::screen_scale()
        }
    }

    /// The level point under a window position in pixels, what a raw
    /// touch carries.
    pub fn convert_touch(pos: Point) -> Point {
        Self::level_point(pos / UIManager::scale())
    }

    /// Render pixels per level unit, the sprite shaders draw ten per unit
    /// times the level scale.
    fn pixels_per_unit() -> f32 {
        10.0 * Self::scale()
    }

    /// The level point under a screen point in UI points, the unit of
    /// `UIManager::cursor_position` and of view frames.
    pub fn level_point(screen: Point) -> Point {
        let center = UIManager::render_area() / 2.0;
        let pixels = screen * UIManager::scale();
        let offset = Point::new(pixels.x - center.width, center.height - pixels.y);
        offset / Self::pixels_per_unit() + *Self::camera_pos()
    }

    /// The screen point in UI points where a level point is drawn, to put
    /// a view over it, like a damage number over an enemy.
    pub fn screen_point(level: impl Into<Point>) -> Point {
        let offset = (level.into() - *Self::camera_pos()) * Self::pixels_per_unit();
        let center = UIManager::render_area() / 2.0;
        Point::new(center.width + offset.x, center.height - offset.y) / UIManager::scale()
    }

    /// The level rect the screen shows.
    pub fn visible_rect() -> Rect {
        let half = UIManager::render_area() / 2.0 / Self::pixels_per_unit();
        let camera = *Self::camera_pos();
        Rect::new(
            camera.x - half.width,
            camera.y - half.height,
            half.width * 2.0,
            half.height * 2.0,
        )
    }
}
