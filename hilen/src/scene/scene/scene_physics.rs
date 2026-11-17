use std::{cell::RefCell, collections::HashMap};

use educe::Educe;
use rapier3d::{
    dynamics::{
        CCDSolver, ImpulseJointSet, IntegrationParameters, IslandManager, MultibodyJointSet, RigidBodyHandle,
        SoftBodySet,
    },
    geometry::{ColliderHandle, CollisionEvent, NarrowPhase},
    pipeline::PhysicsPipeline,
    prelude::BroadPhaseBvh,
};

use crate::{
    deps::refs::{Own, Weak},
    gm::volume::Vec3,
    scene::{GravityWell, Node, event_handler::EventHandler, gravity::gravity_at, sets::Sets},
};

#[derive(Educe)]
#[educe(Default)]
pub(crate) struct ScenePhysics {
    pub(crate) colliding_nodes: HashMap<ColliderHandle, Weak<dyn Node>>,
    /// The node of each collider a query met, see `SceneBase::node_with`. A
    /// scan of every node per hit was the cost of an aim cast each frame.
    pub(crate) query_nodes:     RefCell<HashMap<ColliderHandle, Weak<dyn Node>>>,

    pub(crate) sets: Sets,

    /// The same pull everywhere, 9.81 down by default.
    #[educe(Default = Vec3::new(0.0, -9.81, 0.0))]
    pub(crate) gravity: Vec3,
    /// Masses that pull toward their centers on top of `gravity`.
    pub(crate) wells:   Vec<GravityWell>,
    /// Bodies put to sleep since the last step, see `sleep`.
    sleep_after_step:   Vec<RigidBodyHandle>,

    #[educe(Default = ccd_parameters())]
    pub(crate) integration_parameters: IntegrationParameters,

    physics_pipeline: PhysicsPipeline,

    island_manager:            IslandManager,
    pub(crate) broad_phase:    BroadPhaseBvh,
    pub(crate) narrow_phase:   NarrowPhase,
    pub(crate) impulse_joints: ImpulseJointSet,
    multibody_joints:          MultibodyJointSet,
    soft_bodies:               SoftBodySet,
    ccd_solver:                CCDSolver,

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
    /// The pull of gravity at `point`, the uniform part and every well.
    pub(crate) fn gravity_at(&self, point: Vec3) -> Vec3 {
        gravity_at(self.gravity, &self.wells, point)
    }

    /// Gives every awake dynamic body the pull of the wells at its center
    /// of mass for 1 step. Rapier pulls with 1 uniform vector only. A
    /// sleeping body is left alone, as rapier leaves it under its own
    /// gravity.
    fn pull_by_wells(&mut self, dt: f32) {
        if self.wells.is_empty() {
            return;
        }
        for (_, body) in self.sets.rigid_bodies.iter_mut() {
            if !body.is_dynamic() || body.is_sleeping() {
                continue;
            }
            let center = body.center_of_mass();
            let pull = gravity_at(Vec3::ZERO, &self.wells, Vec3::new(center.x, center.y, center.z));
            body.apply_impulse(pull * (body.mass() * body.gravity_scale() * dt), false);
        }
    }

    /// Puts `body` to sleep, now and once more after the next step. A new
    /// body asleep was woken by the first contact its step found, a ball
    /// made at rest on a slope rolled off.
    pub(crate) fn sleep(&mut self, body: RigidBodyHandle) {
        if let Some(body) = self.sets.rigid_bodies.get_mut(body) {
            body.sleep();
        }
        self.sleep_after_step.push(body);
    }

    pub fn update_physics(&mut self, nodes: &[Own<dyn Node>], frame_time: f32) {
        self.integration_parameters.dt = frame_time;
        self.pull_by_wells(frame_time);

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
            &mut self.soft_bodies,
            &mut self.ccd_solver,
            &(),
            &self.events.handler,
        );
        for body in self.sleep_after_step.drain(..) {
            if let Some(body) = self.sets.rigid_bodies.get_mut(body) {
                body.sleep();
            }
        }

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
                &mut self.soft_bodies,
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
                &mut self.soft_bodies,
                true,
            );
        }
    }
}

#[cfg(test)]
mod test {
    use std::f32::consts::TAU;

    use rapier3d::{
        dynamics::RigidBodyHandle,
        prelude::{ColliderBuilder, RigidBodyBuilder},
    };

    use super::*;
    use crate::gm::{LossyConvert, volume::Quat};

    const DT: f32 = 1.0 / 240.0;

    /// A ball at `position` moving at `speed`, pulled only by `wells`.
    fn space_with_ball(
        position: Vec3,
        speed: Vec3,
        wells: Vec<GravityWell>,
    ) -> (ScenePhysics, RigidBodyHandle) {
        let mut physics = ScenePhysics {
            gravity: Vec3::ZERO,
            wells,
            ..ScenePhysics::default()
        };
        let body = RigidBodyBuilder::dynamic().translation(position).linvel(speed).build();
        let (body, _) = physics.sets.insert(body, ColliderBuilder::ball(0.5).build());
        (physics, body)
    }

    fn place(physics: &ScenePhysics, body: RigidBodyHandle) -> Vec3 {
        let at = physics.sets.rigid_bodies[body].translation();
        Vec3::new(at.x, at.y, at.z)
    }

    /// At the speed of a circle, `sqrt(gm / r)`, a ball goes round a well
    /// at the same distance and comes back in `TAU * r / speed` seconds.
    #[test]
    fn a_ball_at_orbit_speed_circles_a_well() {
        let (gm, radius) = (1000.0_f32, 10.0_f32);
        let speed = (gm / radius).sqrt();
        let well = GravityWell {
            center: Vec3::ZERO,
            gm,
            radius: 1.0,
        };
        let start = Vec3::new(radius, 0.0, 0.0);
        let (mut physics, ball) = space_with_ball(start, Vec3::new(0.0, 0.0, speed), vec![well]);
        let steps: usize = (TAU * radius / speed / DT).round().lossy_convert();
        let (mut nearest, mut farthest) = (f32::MAX, 0.0_f32);
        for _ in 0..steps {
            physics.update_physics(&[], DT);
            let distance = place(&physics, ball).length();
            nearest = nearest.min(distance);
            farthest = farthest.max(distance);
        }
        assert!(
            nearest > radius * 0.99 && farthest < radius * 1.01,
            "the orbit ran from {nearest} to {farthest}"
        );
        let back = place(&physics, ball);
        assert!(
            (back - start).length() < 0.3,
            "after 1 turn the ball is at {back}"
        );
    }

    #[test]
    fn a_sleeping_body_stays_where_it_sleeps() {
        let well = GravityWell::with_surface_gravity(Vec3::new(0.0, -100.0, 0.0), 100.0, 9.81);
        let start = Vec3::new(0.0, 5.0, 0.0);
        let (mut physics, ball) = space_with_ball(start, Vec3::ZERO, vec![well]);
        physics.sets.rigid_bodies[ball].sleep();
        for _ in 0..60 {
            physics.update_physics(&[], DT);
        }
        assert_eq!(place(&physics, ball), start);
    }

    /// A slope of 20 degrees under a planet of a well, and a ball made at
    /// rest on it at `x`, put to sleep at once when `asleep`.
    fn ball_on_slope(physics: &mut ScenePhysics, x: f32, asleep: bool) -> RigidBodyHandle {
        let tilt = Quat::from_rotation_x(0.35);
        let at = tilt * Vec3::new(x, 0.2, 0.0);
        let (ball, _) = physics.sets.insert(
            RigidBodyBuilder::dynamic().translation(at).build(),
            ColliderBuilder::ball(0.1).build(),
        );
        if asleep {
            physics.sleep(ball);
        }
        ball
    }

    // A round stone that a game made a body on a slope rolled off at once.
    #[test]
    fn a_sleeping_ball_stays_on_a_slope_until_a_touch_wakes_it() {
        let tilt = Quat::from_rotation_x(0.35);
        let mut physics = ScenePhysics {
            gravity: Vec3::ZERO,
            wells: vec![GravityWell::with_surface_gravity(
                Vec3::new(0.0, -1000.0, 0.0),
                1000.0,
                9.81,
            )],
            ..ScenePhysics::default()
        };
        let slab = ColliderBuilder::cuboid(10.0, 0.1, 10.0).rotation(tilt.to_scaled_axis()).build();
        physics.sets.colliders.insert(slab);
        let asleep = ball_on_slope(&mut physics, -2.0, true);
        let awake = ball_on_slope(&mut physics, 2.0, false);
        let (asleep_at, awake_at) = (place(&physics, asleep), place(&physics, awake));
        for _ in 0..240 {
            physics.update_physics(&[], DT);
        }
        let stayed = place(&physics, asleep).distance(asleep_at);
        let rolled = place(&physics, awake).distance(awake_at);
        assert!(stayed < 1e-3, "the sleeping ball moved {stayed}");
        assert!(physics.sets.rigid_bodies[asleep].is_sleeping());
        assert!(rolled > 0.5, "the awake ball rolled only {rolled}");

        let up = tilt * Vec3::Y;
        let thrown = RigidBodyBuilder::dynamic()
            .translation(asleep_at + up * 0.6)
            .linvel(-up * 3.0)
            .build();
        physics.sets.insert(thrown, ColliderBuilder::ball(0.1).build());
        for _ in 0..240 {
            physics.update_physics(&[], DT);
        }
        let moved = place(&physics, asleep).distance(asleep_at);
        assert!(moved > 0.05, "the ball that was hit moved only {moved}");
    }
}
