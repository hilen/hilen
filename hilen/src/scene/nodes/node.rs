use std::{
    any::type_name,
    ops::{Deref, DerefMut},
};

use log::error;
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
    scene::{Material, NodeData, Playback, SceneManager, ToCollider},
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
            return Vec3::new(translation.x, translation.y, translation.z) - self.collider_offset();
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

    /// Half the size of the solid along each axis, at the node's scale.
    fn half_extents(&self) -> Vec3 {
        self.shape.half_extents() * self.scale
    }

    /// Where the collider sits relative to the node's origin, at the
    /// node's scale, see `Shape3::collider_offset`.
    fn collider_offset(&self) -> Vec3 {
        self.shape.collider_offset() * self.scale
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
        self.shape().hit(ray, self.position(), self.rotation(), self.scale)
    }

    /// Unit mesh to world, the instance transform the drawer uploads.
    fn model_matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(
            self.shape.mesh_scale() * self.scale,
            self.rotation(),
            self.position(),
        )
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
    /// Sizes the node uniformly on top of its shape, 1 as modeled. A
    /// model in other units fits a scene this way. The collider follows.
    fn set_scale(&mut self, _: impl ToF32) -> &mut Self;
    /// Plays the clip of the node's model called `name` from its
    /// start, looped. A node without a model or a model without that
    /// clip logs an error and keeps drawing at rest.
    fn play(&mut self, name: &str) -> &mut Self;
    /// Plays the clip once and holds its last frame.
    fn play_once(&mut self, name: &str) -> &mut Self;
    /// Clip seconds per scene second for the playing clip, 1 as
    /// authored, negative runs it backwards.
    fn set_animation_speed(&mut self, speed: impl ToF32) -> &mut Self;
    /// Jumps the playing clip to `seconds` from its start, held inside
    /// the clip. Nothing while no clip plays.
    fn set_animation_time(&mut self, seconds: impl ToF32) -> &mut Self;
    /// Back to the rest pose.
    fn stop_animation(&mut self) -> &mut Self;
    /// Seconds into the playing clip, `None` while nothing plays. A
    /// clip played once still counts as playing on its last frame.
    fn animation_time(&self) -> Option<f32>;
    /// Whether a clip is moving the node, false once a clip played once
    /// has reached its end.
    fn is_animating(&self) -> bool;
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
            let offset = self.collider_offset();
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

    fn set_scale(&mut self, scale: impl ToF32) -> &mut Self {
        self.scale = scale.to_f32();
        if self.collider_handle().is_some() {
            let shape = self.shape.make_collider(self.scale).shape;
            let offset = self.collider_offset();
            let body = self.rigid_handle().is_some();
            let position = self.position;
            let collider = self.collider_mut();
            collider.set_shape(shape);
            if body {
                collider.set_position_wrt_parent(Pose3::from_translation(offset));
            } else {
                collider.set_position(Pose3::from_translation(position + offset));
            }
        }
        self
    }

    fn play(&mut self, name: &str) -> &mut Self {
        self.playback = clip_of(self.shape, name).map(|clip| Playback::new(clip, true));
        self
    }

    fn play_once(&mut self, name: &str) -> &mut Self {
        self.playback = clip_of(self.shape, name).map(|clip| Playback::new(clip, false));
        self
    }

    fn set_animation_speed(&mut self, speed: impl ToF32) -> &mut Self {
        if let Some(playback) = self.playback.as_mut() {
            playback.speed = speed.to_f32();
        }
        self
    }

    fn set_animation_time(&mut self, seconds: impl ToF32) -> &mut Self {
        let duration = clip_duration(self.shape, self.playback);
        if let (Some(playback), Some(duration)) = (self.playback.as_mut(), duration) {
            playback.seek(seconds.to_f32(), duration);
        }
        self
    }

    fn stop_animation(&mut self) -> &mut Self {
        self.playback = None;
        self
    }

    fn animation_time(&self) -> Option<f32> {
        self.playback.map(|playback| playback.time)
    }

    fn is_animating(&self) -> bool {
        match (self.playback, clip_duration(self.shape, self.playback)) {
            (Some(playback), Some(duration)) => !playback.finished(duration),
            _ => false,
        }
    }
}

/// The length of the clip a node plays, `None` without a model or a clip.
fn clip_duration(shape: Shape3, playback: Option<Playback>) -> Option<f32> {
    let (Shape3::Model(model), Some(playback)) = (shape, playback) else {
        return None;
    };
    model.is_ok().then(|| model.clips()[playback.clip].duration)
}

/// The index of the clip called `name` in the node's model, logged when
/// there is no such clip or no model.
fn clip_of(shape: Shape3, name: &str) -> Option<usize> {
    let Shape3::Model(model) = shape else {
        error!("play({name}): the node draws no model");
        return None;
    };
    if !model.is_ok() {
        error!("play({name}): the model is gone");
        return None;
    }
    let clip = model.clip(name);
    if clip.is_none() {
        error!("play({name}): the model has no such clip");
    }
    clip
}
