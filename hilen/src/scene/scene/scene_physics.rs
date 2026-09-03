use std::collections::HashMap;

use educe::Educe;
use rapier3d::{
    dynamics::{CCDSolver, ImpulseJointSet, IntegrationParameters, IslandManager, MultibodyJointSet},
    geometry::{ColliderHandle, CollisionEvent, NarrowPhase},
    pipeline::PhysicsPipeline,
    prelude::BroadPhaseBvh,
};

use crate::{
    deps::refs::{Own, Weak},
    gm::volume::Vec3,
    scene::{Node, event_handler::EventHandler, sets::Sets},
};

#[derive(Educe)]
#[educe(Default)]
pub(crate) struct ScenePhysics {
    pub(crate) colliding_nodes: HashMap<ColliderHandle, Weak<dyn Node>>,

    pub(crate) sets: Sets,

    #[educe(Default = Vec3::new(0.0, -9.81, 0.0))]
    pub(crate) gravity: Vec3,

    #[educe(Default = ccd_parameters())]
    integration_parameters: IntegrationParameters,

    physics_pipeline: PhysicsPipeline,

    island_manager:          IslandManager,
    pub(crate) broad_phase:  BroadPhaseBvh,
    pub(crate) narrow_phase: NarrowPhase,
    impulse_joints:          ImpulseJointSet,
    multibody_joints:        MultibodyJointSet,
    ccd_solver:              CCDSolver,

    pub(crate) events: EventHandler,
}

// The same as a level, one CCD substep lets a fast body settle inside
// the wall it should have bounced off.
fn ccd_parameters() -> IntegrationParameters {
    IntegrationParameters {
        max_ccd_substeps: 4,
        ..IntegrationParameters::default()
    }
}

impl ScenePhysics {
    pub fn update_physics(&mut self, nodes: &[Own<dyn Node>], frame_time: f32) {
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

        self.handle_collisions(nodes);
    }

    fn node_with_collider(
        nodes: &[Own<dyn Node>],
        collider_handle: ColliderHandle,
    ) -> Option<Weak<dyn Node>> {
        nodes
            .iter()
            .find(|a| match a.collider_handle() {
                Some(handle) => handle == collider_handle,
                None => false,
            })
            .map(Own::weak)
    }

    fn handle_collisions(&self, nodes: &[Own<dyn Node>]) {
        while let Ok(contact) = self.events.intersection.try_recv() {
            let CollisionEvent::Started(a, b, _) = contact else {
                continue;
            };

            let a = self
                .colliding_nodes
                .get(&a)
                .copied()
                .unwrap_or_else(|| Self::node_with_collider(nodes, a).unwrap());

            let b = self
                .colliding_nodes
                .get(&b)
                .copied()
                .unwrap_or_else(|| Self::node_with_collider(nodes, b).unwrap());

            if a.collision_enabled {
                a.on_collision.trigger(b);
            }

            if b.collision_enabled {
                b.on_collision.trigger(a);
            }
        }
    }

    pub(crate) fn remove(&mut self, node: &dyn Node) {
        if let Some(collider) = node.collider_handle() {
            self.sets.colliders.remove(
                collider,
                &mut self.island_manager,
                &mut self.sets.rigid_bodies,
                true,
            );
        }

        if let Some(rigid_body) = node.rigid_handle() {
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
