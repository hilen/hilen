use std::ops::{Deref, DerefMut};

use rapier3d::{dynamics::RigidBodyHandle, geometry::ColliderHandle, prelude::RigidBodyBuilder};

use crate::{
    deps::refs::{Own, Weak, weak_from_ref},
    gm::volume::{Shape3, Vec3},
    scene::{Node, NodeData, SceneManager, ToCollider},
};

/// A dynamic body, moved by gravity, contacts and impulses.
pub struct Body {
    rigid_handle:    RigidBodyHandle,
    collider_handle: ColliderHandle,
    node:            NodeData,
}

impl Body {
    pub fn velocity(&self) -> Vec3 {
        let vel = self.rigid_body().linvel();
        Vec3::new(vel.x, vel.y, vel.z)
    }

    pub fn set_velocity(&mut self, vel: Vec3) -> &mut Self {
        self.rigid_body_mut().set_linvel(vel, true);
        self
    }

    /// The turn speed, radians per second about each axis of the world.
    pub fn angular_velocity(&self) -> Vec3 {
        self.rigid_body().angvel()
    }

    pub fn set_angular_velocity(&mut self, vel: Vec3) -> &mut Self {
        self.rigid_body_mut().set_angvel(vel, true);
        self
    }

    pub fn mass(&self) -> f32 {
        self.rigid_body().mass()
    }

    /// The mass in kg, in place of the one the collider's volume gives.
    /// A new collider from `set_collider` takes its own mass again.
    pub fn set_mass(&mut self, mass: f32) -> &mut Self {
        self.collider_mut().set_mass(mass);
        self
    }

    /// How much of the scene's gravity pulls this body, 1 by default. With
    /// 0 it floats, like a hand that a game moves by its speed.
    pub fn set_gravity_scale(&mut self, scale: f32) -> &mut Self {
        self.rigid_body_mut().set_gravity_scale(scale, true);
        self
    }

    pub fn add_impulse(&mut self, impulse: Vec3) -> &mut Self {
        self.rigid_body_mut().apply_impulse(impulse, true);
        self
    }

    /// How fast motion dies out on its own, per second. Rapier has no
    /// rolling resistance, so a ball on a plane rolls forever without
    /// angular damping.
    pub fn set_damping(&mut self, linear: f32, angular: f32) -> &mut Self {
        let body = self.rigid_body_mut();
        body.set_linear_damping(linear);
        body.set_angular_damping(angular);
        self
    }
}

impl Node for Body {
    fn make(shape: Shape3, position: Vec3) -> Own<Self>
    where Self: Sized {
        // The same rule as a level body, a fast body must not cross a
        // thin wall in one step.
        let rigid_body = RigidBodyBuilder::dynamic().translation(position).ccd_enabled(true).build();

        // The collider sits where the shape's solid is, a model's box
        // around its bounds, the same offset a wall applies.
        let collider = shape.make_collider(1.0).translation(shape.collider_offset()).build();

        let (rigid_handle, collider_handle) = SceneManager::physics().sets.insert(rigid_body, collider);

        Own::new(Self {
            rigid_handle,
            collider_handle,
            node: NodeData::make(shape, position),
        })
    }

    fn rigid_handle(&self) -> Option<RigidBodyHandle> {
        self.rigid_handle.into()
    }

    fn collider_handle(&self) -> Option<ColliderHandle> {
        self.collider_handle.into()
    }

    fn weak_node(&self) -> Weak<dyn Node> {
        weak_from_ref(self)
    }
}

impl Deref for Body {
    type Target = NodeData;
    fn deref(&self) -> &NodeData {
        &self.node
    }
}

impl DerefMut for Body {
    fn deref_mut(&mut self) -> &mut NodeData {
        &mut self.node
    }
}
