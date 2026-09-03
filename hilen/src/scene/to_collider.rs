use rapier3d::prelude::ColliderBuilder;

use crate::gm::volume::Shape3;

pub trait ToCollider {
    fn make_collider(&self) -> ColliderBuilder;
}

impl ToCollider for Shape3 {
    fn make_collider(&self) -> ColliderBuilder {
        match self {
            Shape3::Ball(radius) => ColliderBuilder::ball(*radius),
            Shape3::Box(_) | Shape3::Plane(_) | Shape3::Model(_) => {
                let half = self.half_extents();
                ColliderBuilder::cuboid(half.x, half.y, half.z)
            }
        }
    }
}
