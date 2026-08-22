use std::collections::HashMap;

use educe::Educe;
use rapier2d::{
    dynamics::{CCDSolver, ImpulseJointSet, IntegrationParameters, IslandManager, MultibodyJointSet},
    geometry::{ColliderHandle, CollisionEvent, NarrowPhase},
    pipeline::PhysicsPipeline,
    prelude::{BroadPhaseBvh, Vec2},
};

use crate::{
    deps::refs::{Own, Weak},
    level::{Sprite, event_handler::EventHandler, sets::Sets},
};

#[derive(Educe)]
#[educe(Default)]
pub(crate) struct LevelPhysics {
    pub(crate) colliding_sprites: HashMap<ColliderHandle, Weak<dyn Sprite>>,

    pub(crate) sets: Sets,

    #[educe(Default = Vec2::new(0.0, -9.81))]
    pub(crate) gravity: Vec2,

    integration_parameters: IntegrationParameters,

    physics_pipeline: PhysicsPipeline,

    island_manager:   IslandManager,
    broad_phase:      BroadPhaseBvh,
    narrow_phase:     NarrowPhase,
    impulse_joints:   ImpulseJointSet,
    multibody_joints: MultibodyJointSet,
    ccd_solver:       CCDSolver,

    pub(crate) events: EventHandler,
}

impl LevelPhysics {
    pub fn update_physics(&mut self, sprites: &[Own<dyn Sprite>], frame_time: f32) {
        self.integration_parameters.dt = frame_time;

        self.physics_pipeline.step(
            self.gravity,
            &self.integration_parameters,
            &mut self.island_manager,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.sets.rigid_bodies,
            &mut self.sets.colliders,
            &mut self.impulse_joints,
            &mut self.multibody_joints,
            &mut self.ccd_solver,
            &(),
            &self.events.handler,
        );

        self.handle_collisions(sprites);
    }

    fn sprite_with_collider(
        sprites: &[Own<dyn Sprite>],
        collider_handle: ColliderHandle,
    ) -> Option<Weak<dyn Sprite>> {
        sprites
            .iter()
            .find(|a| match a.collider_handle() {
                Some(handle) => handle == collider_handle,
                None => false,
            })
            .map(Own::weak)
    }

    fn handle_collisions(&self, sprites: &[Own<dyn Sprite>]) {
        while let Ok(contact) = self.events.intersection.try_recv() {
            let CollisionEvent::Started(a, b, _) = contact else {
                continue;
            };

            let a = self
                .colliding_sprites
                .get(&a)
                .copied()
                .unwrap_or_else(|| Self::sprite_with_collider(sprites, a).unwrap());

            let b = self
                .colliding_sprites
                .get(&b)
                .copied()
                .unwrap_or_else(|| Self::sprite_with_collider(sprites, b).unwrap());

            if a.collision_enabled {
                a.on_collision.trigger(b);
            }

            if b.collision_enabled {
                b.on_collision.trigger(a);
            }
        }
    }

    pub(crate) fn remove(&mut self, sprite: &dyn Sprite) {
        if let Some(collider) = sprite.collider_handle() {
            self.sets.colliders.remove(
                collider,
                &mut self.island_manager,
                &mut self.sets.rigid_bodies,
                true,
            );
        }

        if let Some(rigid_body) = sprite.rigid_handle() {
            self.sets.rigid_bodies.remove(
                rigid_body,
                &mut self.island_manager,
                &mut self.sets.colliders,
                &mut self.impulse_joints,
                &mut self.multibody_joints,
                true,
            );
        }
    }
}
