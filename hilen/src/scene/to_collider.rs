use rapier3d::prelude::ColliderBuilder;

use crate::gm::volume::Shape3;

pub trait ToCollider {
    /// The collider of the shape at a node's `scale`.
    fn make_collider(&self, scale: f32) -> ColliderBuilder;
}

impl ToCollider for Shape3 {
    fn make_collider(&self, scale: f32) -> ColliderBuilder {
        match self {
            Shape3::Ball(radius) => ColliderBuilder::ball(*radius * scale),
            Shape3::Box(_) | Shape3::Plane(_) | Shape3::Model(_) => {
                let half = self.half_extents() * scale;
                ColliderBuilder::cuboid(half.x, half.y, half.z)
            }
        }
    }
}
