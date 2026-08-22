use std::ops::{Deref, DerefMut};

use rapier2d::{geometry::ColliderHandle, prelude::Vec2};

use crate::{
    deps::refs::{Own, Weak, weak_from_ref},
    gm::flat::{Point, Shape},
    level::{LevelManager, Sprite, SpriteData, ToCollider},
};

pub struct Wall {
    collider_handle: ColliderHandle,
    sprite:          SpriteData,
}

impl Sprite for Wall {
    fn make(shape: Shape, position: Point) -> Own<Self> {
        let collider = shape
            .make_collider()
            .translation(Vec2::new(position.x, position.y))
            .restitution(1.0)
            .build();

        let sprite = SpriteData::make(shape, position);
        let collider_handle = LevelManager::physics().sets.colliders.insert(collider);

        Own::new(Wall {
            collider_handle,
            sprite,
        })
    }

    fn collider_handle(&self) -> Option<ColliderHandle> {
        self.collider_handle.into()
    }

    fn weak_sprite(&self) -> Weak<dyn Sprite> {
        weak_from_ref(self)
    }
}

impl Deref for Wall {
    type Target = SpriteData;

    fn deref(&self) -> &Self::Target {
        &self.sprite
    }
}

impl DerefMut for Wall {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.sprite
    }
}
