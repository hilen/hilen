use std::ops::{Deref, DerefMut};

use rapier2d::{
    dynamics::RigidBodyHandle,
    geometry::ColliderHandle,
    prelude::{RigidBodyBuilder, Vec2},
};

use crate::{
    deps::refs::{Own, Weak, weak_from_ref},
    gm::flat::{Point, Shape},
    level::{LevelManager, Sprite, SpriteData, ToCollider, control::Control},
};

pub struct Body {
    rigid_handle:    RigidBodyHandle,
    collider_handle: ColliderHandle,
    sprite:          SpriteData,

    pub jump_force: f32,
}

impl Body {
    pub(crate) fn velocity(&self) -> Point {
        let vel = self.rigid_body().linvel();
        (vel.x, vel.y).into()
    }

    pub(crate) fn set_velocity(&mut self, vel: Point) -> &mut Self {
        self.rigid_body_mut().set_linvel(Vec2::new(vel.x, vel.y), true);
        self
    }
}

impl Sprite for Body {
    fn make(shape: Shape, position: Point) -> Own<Self>
    where Self: Sized {
        // A body pushed at speed by a moving wall can cross a thin wall in
        // one step, so every body gets continuous collision detection.
        let rigid_body = RigidBodyBuilder::dynamic()
            .translation(Vec2::new(position.x, position.y))
            .ccd_enabled(true)
            .build();

        let collider = shape.make_collider().build();

        let (rigid_handle, collider_handle) = LevelManager::physics().sets.insert(rigid_body, collider);

        let sprite = SpriteData::make(shape, position);

        Own::new(Self {
            rigid_handle,
            collider_handle,
            sprite,
            jump_force: 0.1,
        })
    }

    fn weak_sprite(&self) -> Weak<dyn Sprite> {
        weak_from_ref(self)
    }

    fn rigid_handle(&self) -> Option<RigidBodyHandle> {
        self.rigid_handle.into()
    }

    fn collider_handle(&self) -> Option<ColliderHandle> {
        self.collider_handle.into()
    }
}

impl Control for Body {
    fn jump(&mut self) {
        self.add_impulse((0, self.jump_force).into());
    }

    fn go_left(&mut self) {
        self.add_impulse((-self.jump_force, 0).into());
    }

    fn go_right(&mut self) {
        self.add_impulse((self.jump_force, 0).into());
    }

    fn go_down(&mut self) {
        self.add_impulse((0, -self.jump_force).into());
    }

    fn add_impulse(&mut self, impulse: Point) {
        self.rigid_body_mut()
            .apply_impulse(Vec2::new(impulse.x * 100.0, impulse.y * 100.0), true);
    }
}

impl Deref for Body {
    type Target = SpriteData;
    fn deref(&self) -> &SpriteData {
        &self.sprite
    }
}

impl DerefMut for Body {
    fn deref_mut(&mut self) -> &mut SpriteData {
        &mut self.sprite
    }
}
