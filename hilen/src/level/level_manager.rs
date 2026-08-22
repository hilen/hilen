use std::ops::{Deref, DerefMut};

use educe::Educe;
use plat::Platform;
use rapier2d::{
    dynamics::{RigidBody, RigidBodyHandle},
    prelude::{Collider, ColliderHandle},
};

use crate::{
    deps::refs::{Own, Weak, main_lock::MainLock},
    gm::flat::{Point, Size},
    level::{Level, level::LevelPhysics},
    window::Window,
};

static SELF: MainLock<LevelManager> = MainLock::new();

#[derive(Educe)]
#[educe(Default)]
pub struct LevelManager {
    #[educe(Default = 1.0)]
    scale:      f32,
    camera_pos: Point,

    #[educe(Default = 1.0 / 60.0)]
    update_interval: f32,

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

    pub(crate) fn update() {
        if Self::no_level() {
            return;
        }

        Self::level().__internal_update(*Self::update_interval());
    }
}

impl LevelManager {
    pub fn set_level<T: Level + 'static>(level: T) -> Weak<T> {
        let l = SELF.get_mut();
        let level = Own::new(level);
        let weak = level.weak();
        l.level = Some(level);
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

    pub(crate) fn update_interval() -> &'static mut f32 {
        &mut SELF.get_mut().update_interval
    }

    pub fn camera_pos() -> &'static mut Point {
        &mut SELF.get_mut().camera_pos
    }

    pub fn convert_touch(pos: Point) -> Point {
        let mut pos = pos;
        let size = Window::inner_size();
        let size: Size = (size.width, size.height).into();

        pos.x -= size.width / 2.0;
        pos.y -= size.height / 2.0;
        pos.y = -pos.y;
        pos /= 10.0;

        pos *= 2;

        if Platform::WINDOWS {
            pos /= Window::screen_scale().ceil();
        } else {
            pos /= Window::screen_scale();
        }

        pos /= Self::scale();

        pos += *Self::camera_pos();

        pos
    }
}
