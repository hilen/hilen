use std::ops::{Deref, DerefMut};

use rapier3d::geometry::ColliderHandle;

use crate::{
    deps::refs::{Own, Weak, weak_from_ref},
    gm::volume::{Shape3, Vec3},
    scene::{Node, NodeData, SceneManager, ToCollider},
};

/// A fixed collider that bodies rest on and bounce off. A floor is a
/// wall with a plane shape.
pub struct Wall {
    collider_handle: ColliderHandle,
    node:            NodeData,
}

impl Node for Wall {
    fn make(shape: Shape3, position: Vec3) -> Own<Self> {
        // Bouncy like a level wall, so a ball rolls on instead of
        // stopping dead against it.
        let collider = shape
            .make_collider(1.0)
            .translation(position + shape.collider_offset())
            .restitution(1.0)
            .build();

        let collider_handle = SceneManager::physics().sets.colliders.insert(collider);

        Own::new(Wall {
            collider_handle,
            node: NodeData::make(shape, position),
        })
    }

    fn collider_handle(&self) -> Option<ColliderHandle> {
        self.collider_handle.into()
    }

    fn weak_node(&self) -> Weak<dyn Node> {
        weak_from_ref(self)
    }
}

impl Deref for Wall {
    type Target = NodeData;

    fn deref(&self) -> &Self::Target {
        &self.node
    }
}

impl DerefMut for Wall {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.node
    }
}
