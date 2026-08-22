use rapier2d::{
    dynamics::{RigidBody, RigidBodyHandle},
    geometry::{Collider, ColliderHandle},
    prelude::{ColliderSet, RigidBodySet},
};

#[derive(Default)]
pub(crate) struct Sets {
    pub rigid_bodies: RigidBodySet,
    pub colliders:    ColliderSet,
}

impl Sets {
    pub(crate) fn insert(
        &mut self,
        rigid: RigidBody,
        collider: Collider,
    ) -> (RigidBodyHandle, ColliderHandle) {
        let rigid_handle = self.rigid_bodies.insert(rigid);

        let collider_handle =
            self.colliders
                .insert_with_parent(collider, rigid_handle, &mut self.rigid_bodies);

        (rigid_handle, collider_handle)
    }
}
