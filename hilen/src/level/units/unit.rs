use std::ops::{Deref, DerefMut};

use rapier2d::{
    dynamics::{CoefficientCombineRule, RigidBodyHandle},
    geometry::ColliderHandle,
};

use crate::{
    deps::refs::{Own, Weak, weak_from_ref},
    gm::flat::{Point, Shape},
    level::{Body, Sprite, SpriteData},
};

pub struct Unit {
    pub body: Own<Body>,
}

impl Sprite for Unit {
    fn make(shape: Shape, position: Point) -> Own<Self>
    where Self: Sized {
        let mut body = Body::make(shape, position);

        body.lock_rotations();
        body.collider_mut().set_restitution(0.0);
        body.collider_mut().set_restitution_combine_rule(CoefficientCombineRule::Min);

        Own::new(Unit { body })
    }

    fn update(&mut self) {
        self.body.update();
    }

    fn rigid_handle(&self) -> Option<RigidBodyHandle> {
        self.body.rigid_handle()
    }

    fn collider_handle(&self) -> Option<ColliderHandle> {
        self.body.collider_handle()
    }

    fn weak_sprite(&self) -> Weak<dyn Sprite> {
        weak_from_ref(self)
    }
}

impl Deref for Unit {
    type Target = SpriteData;
    fn deref(&self) -> &SpriteData {
        &self.body
    }
}

impl DerefMut for Unit {
    fn deref_mut(&mut self) -> &mut SpriteData {
        &mut self.body
    }
}
