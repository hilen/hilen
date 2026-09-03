use std::{
    any::type_name,
    ops::{Deref, DerefMut},
};

use rapier3d::{
    dynamics::RigidBodyHandle,
    geometry::{Collider, ColliderHandle},
    pipeline::ActiveEvents,
    prelude::{CoefficientCombineRule, Pose3, RigidBody},
};

use crate::{
    deps::refs::{Own, Weak, weak_from_ref},
    gm::{
        ToF32,
        color::Color,
        volume::{Mat4, Quat, Ray, Shape3, Vec3},
    },
    scene::{Material, NodeData, SceneManager},
};

pub trait Node: Deref<Target = NodeData> + DerefMut {
    fn make(shape: Shape3, position: Vec3) -> Own<Self>
    where Self: Sized;

    fn update(&mut self) {}

    fn shape(&self) -> Shape3 {
        self.shape
    }

    fn rigid_handle(&self) -> Option<RigidBodyHandle> {
        None
    }

    fn collider_handle(&self) -> Option<ColliderHandle> {
        None
    }

    fn position(&self) -> Vec3 {
        if let Some(handle) = self.rigid_handle() {
            let translation = SceneManager::get_rigid_body(handle).translation();
            return Vec3::new(translation.x, translation.y, translation.z);
        }
        if let Some(handle) = self.collider_handle() {
            let translation = SceneManager::get_collider(handle).translation();
            return Vec3::new(translation.x, translation.y, translation.z) - self.shape.collider_offset();
        }
        self.position
    }

    fn rotation(&self) -> Quat {
        if let Some(handle) = self.rigid_handle() {
            let rotation = SceneManager::get_rigid_body(handle).rotation();
            Quat::from_xyzw(rotation.x, rotation.y, rotation.z, rotation.w)
        } else if let Some(handle) = self.collider_handle() {
            let rotation = SceneManager::get_collider(handle).rotation();
            Quat::from_xyzw(rotation.x, rotation.y, rotation.z, rotation.w)
        } else {
            self.rotation
        }
    }

    fn restitution(&self) -> f32 {
        self.collider().restitution()
    }

    fn rigid_body(&self) -> &RigidBody {
        &SceneManager::physics().sets.rigid_bodies
            [self.rigid_handle().expect("This node doesn't have rigid body")]
    }

    fn rigid_body_mut(&mut self) -> &mut RigidBody {
        let handle = self.rigid_handle().expect("This node doesn't have rigid body");
        &mut SceneManager::physics().sets.rigid_bodies[handle]
    }

    fn collider(&self) -> &Collider {
        &SceneManager::physics().sets.colliders
            [self.collider_handle().expect("This node doesn't have collider")]
    }

    fn collider_mut(&mut self) -> &mut Collider {
        let handle = self.collider_handle().expect("This node doesn't have collider");
        &mut SceneManager::physics().sets.colliders[handle]
    }

    fn enable_collision_detection(&mut self)
    where Self: Sized + 'static {
        assert!(
            self.collider_handle().is_some(),
            "{} doesn't have a collider.",
            type_name::<Self>()
        );
        self.collision_enabled = true;
        self.collider_mut().set_active_events(ActiveEvents::COLLISION_EVENTS);
        let weak = weak_from_ref(self);
        SceneManager::physics()
            .colliding_nodes
            .insert(weak.collider_handle().unwrap(), weak);
    }

    fn color(&self) -> &Color {
        &self.material.color
    }

    fn remove(&mut self) {
        SceneManager::scene_weak().remove(self.weak_node());
    }

    fn lock_rotations(&mut self) {
        if self.rigid_handle().is_some() {
            self.rigid_body_mut().lock_rotations(true, true);
        }
    }

    fn unlock_rotations(&mut self) {
        if self.rigid_handle().is_some() {
            self.rigid_body_mut().lock_rotations(false, true);
        }
    }

    /// The distance along the ray to this node's solid, see `Shape3::hit`.
    fn hit(&self, ray: Ray) -> Option<f32> {
        self.shape().hit(ray, self.position(), self.rotation())
    }

    /// Unit mesh to world, the instance transform the drawer uploads.
    fn model_matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.shape.mesh_scale(), self.rotation(), self.position())
    }

    fn weak_node(&self) -> Weak<dyn Node>;
}

pub trait NodeTemplates {
    fn set_color(&mut self, _: Color) -> &mut Self;
    fn set_material(&mut self, _: Material) -> &mut Self;
    fn set_metallic(&mut self, _: impl ToF32) -> &mut Self;
    fn set_roughness(&mut self, _: impl ToF32) -> &mut Self;
    fn set_friction(&mut self, friction: impl ToF32) -> &mut Self;
    fn set_restitution(&mut self, _: f32, _: CoefficientCombineRule) -> &mut Self;
    fn set_position(&mut self, _: impl Into<Vec3>) -> &mut Self;
    fn set_rotation(&mut self, _: Quat) -> &mut Self;
}

impl<T: ?Sized + Node> NodeTemplates for T {
    fn set_color(&mut self, color: Color) -> &mut Self {
        self.material.color = color;
        self
    }

    fn set_material(&mut self, material: Material) -> &mut Self {
        self.material = material;
        self
    }

    fn set_metallic(&mut self, metallic: impl ToF32) -> &mut Self {
        self.material.metallic = metallic.to_f32();
        self
    }

    fn set_roughness(&mut self, roughness: impl ToF32) -> &mut Self {
        self.material.roughness = roughness.to_f32();
        self
    }

    fn set_friction(&mut self, friction: impl ToF32) -> &mut Self {
        self.collider_mut().set_friction(friction.to_f32());
        self
    }

    fn set_restitution(&mut self, res: f32, rule: CoefficientCombineRule) -> &mut Self {
        self.collider_mut().set_restitution(res);
        self.collider_mut().set_restitution_combine_rule(rule);
        self
    }

    /// A body moves through its rigid body, the collider follows. A
    /// standalone collider moves itself.
    fn set_position(&mut self, pos: impl Into<Vec3>) -> &mut Self {
        let pos = pos.into();
        if self.rigid_handle().is_some() {
            self.rigid_body_mut().set_position(Pose3::from_translation(pos), true);
        } else if self.collider_handle().is_some() {
            let offset = self.shape.collider_offset();
            self.collider_mut().set_position(Pose3::from_translation(pos + offset));
        }
        self.position = pos;
        self
    }

    fn set_rotation(&mut self, rotation: Quat) -> &mut Self {
        if self.rigid_handle().is_some() {
            self.rigid_body_mut().set_rotation(rotation, true);
        } else if self.collider_handle().is_some() {
            self.collider_mut().set_rotation(rotation);
        }
        self.rotation = rotation;
        self
    }
}
