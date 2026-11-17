use rapier3d::prelude::ColliderBuilder;

use crate::{gm::volume::Shape3, scene::ColliderShape};

pub trait ToCollider {
    /// The collider of the shape at a node's `scale`.
    fn make_collider(&self, scale: f32) -> ColliderBuilder;
}

impl ToCollider for Shape3 {
    fn make_collider(&self, scale: f32) -> ColliderBuilder {
        ColliderShape::of(*self).0.builder(scale)
    }
}

impl ToCollider for ColliderShape {
    fn make_collider(&self, scale: f32) -> ColliderBuilder {
        self.builder(scale)
    }
}
