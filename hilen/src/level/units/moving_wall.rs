use std::ops::{Deref, DerefMut};

use rapier2d::{
    dynamics::RigidBodyHandle,
    geometry::ColliderHandle,
    prelude::{RigidBodyBuilder, Vec2},
};

use crate::{
    deps::refs::{Own, Weak, weak_from_ref},
    gm::flat::{Point, Shape},
    level::{LevelManager, Sprite, SpriteData, ToCollider},
};

pub struct MovingWall {
    rigid_handle:    RigidBodyHandle,
    collider_handle: ColliderHandle,
    sprite:          SpriteData,
}

impl Sprite for MovingWall {
    fn make(shape: Shape, position: Point) -> Own<Self>
    where Self: Sized {
        let rigid_body = RigidBodyBuilder::kinematic_position_based()
            .translation(Vec2::new(position.x, position.y))
            .build();

        let collider = shape.make_collider().restitution(1.0).build();

        let (rigid_handle, collider_handle) = LevelManager::physics().sets.insert(rigid_body, collider);

        let sprite = SpriteData::make(shape, position);

        Own::new(MovingWall {
            rigid_handle,
            collider_handle,
            sprite,
        })
    }

    fn rigid_handle(&self) -> Option<RigidBodyHandle> {
        self.rigid_handle.into()
    }

    fn collider_handle(&self) -> Option<ColliderHandle> {
        self.collider_handle.into()
    }

    fn weak_sprite(&self) -> Weak<dyn Sprite> {
        weak_from_ref(self)
    }

    fn position(&self) -> Point {
        let rb = LevelManager::get_rigid_body(self.rigid_handle);
        let t = rb.translation();
        Point::new(t.x, t.y)
    }
}

impl MovingWall {
    pub(crate) fn move_to(&mut self, pos: Point) {
        self.rigid_body_mut().set_next_kinematic_translation(Vec2::new(pos.x, pos.y));
        self.sprite.position = pos;
    }

    pub fn move_x(&mut self, x: f32) {
        let mut pos = self.position();
        pos.x = x;
        self.move_to(pos);
    }

    pub fn move_y(&mut self, y: f32) {
        let mut pos = self.position();
        pos.y = y;
        self.move_to(pos);
    }
}

impl Deref for MovingWall {
    type Target = SpriteData;

    fn deref(&self) -> &Self::Target {
        &self.sprite
    }
}

impl DerefMut for MovingWall {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.sprite
    }
}
