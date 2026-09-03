use std::f32::consts::FRAC_PI_2;

use rapier3d::{
    control::{CharacterAutostep, CharacterLength, KinematicCharacterController},
    dynamics::RigidBodyHandle,
    geometry::ColliderHandle,
    pipeline::QueryFilter,
    prelude::{ColliderBuilder, RigidBodyBuilder},
};

use crate::{
    gm::volume::Vec3,
    scene::{SceneManager, scene::ScenePhysics},
    ui::{Cursor, Keys},
    window::KeyCode,
};

/// Looking straight up or down flips the view, so the pitch stops just
/// short of it.
const PITCH_LIMIT: f32 = FRAC_PI_2 - 0.05;

/// A first person player. A capsule the rapier character controller
/// walks over the scene's colliders, with gravity, small steps, a jump
/// and a push on the bodies it walks into, and the scene camera looks
/// out of its eyes. `w` `a` `s` `d` or the arrows walk, space jumps,
/// the captured mouse or `look` turns.
pub struct Player {
    body:       RigidBodyHandle,
    collider:   ColliderHandle,
    controller: KinematicCharacterController,

    /// Radians around the up axis, 0 looks down `-z`, positive turns
    /// right.
    pub yaw:        f32,
    /// Radians up from level.
    pub pitch:      f32,
    /// Units per second.
    pub speed:      f32,
    /// The upward speed a jump starts with.
    pub jump_speed: f32,
    /// The eye above the capsule's center.
    pub eye_height: f32,
    /// What the player weighs when it pushes a body.
    pub mass:       f32,
    /// Reads the keyboard every step. Off for a scene that moves the
    /// player itself.
    pub keyboard:   bool,
    /// Turns with the captured mouse every step, see `Cursor`. Off for
    /// a scene that turns the player itself.
    pub mouse:      bool,
    /// Radians of turn per unit of mouse motion.
    pub look_speed: f32,

    vertical: f32,
    grounded: bool,
}

impl Player {
    pub(crate) fn make(physics: &mut ScenePhysics, position: Vec3, radius: f32, height: f32) -> Self {
        let half_height = (height / 2.0 - radius).max(0.0);
        let body = RigidBodyBuilder::kinematic_position_based().translation(position).build();
        let collider = ColliderBuilder::capsule_y(half_height, radius).build();
        let (body, collider) = physics.sets.insert(body, collider);

        let controller = KinematicCharacterController {
            offset: CharacterLength::Absolute(0.02),
            autostep: Some(CharacterAutostep {
                max_height:             CharacterLength::Absolute(0.35),
                min_width:              CharacterLength::Absolute(0.2),
                include_dynamic_bodies: true,
            }),
            snap_to_ground: Some(CharacterLength::Absolute(0.2)),
            ..KinematicCharacterController::default()
        };

        Self {
            body,
            collider,
            controller,
            yaw: 0.0,
            pitch: 0.0,
            speed: 4.0,
            jump_speed: 5.0,
            eye_height: half_height + radius - 0.15,
            mass: 70.0,
            keyboard: true,
            mouse: true,
            look_speed: 0.002,
            vertical: 0.0,
            grounded: false,
        }
    }

    /// Turn by `yaw` and `pitch` radians.
    pub fn look(&mut self, yaw: f32, pitch: f32) {
        self.yaw += yaw;
        self.pitch = (self.pitch + pitch).clamp(-PITCH_LIMIT, PITCH_LIMIT);
    }

    /// Where the player looks, unit length.
    pub fn direction(&self) -> Vec3 {
        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();
        let (sin_pitch, cos_pitch) = self.pitch.sin_cos();
        Vec3::new(sin_yaw * cos_pitch, sin_pitch, -cos_yaw * cos_pitch)
    }

    fn forward(&self) -> Vec3 {
        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();
        Vec3::new(sin_yaw, 0.0, -cos_yaw)
    }

    fn right(&self) -> Vec3 {
        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();
        Vec3::new(cos_yaw, 0.0, sin_yaw)
    }

    /// The center of the capsule.
    pub fn position(&self) -> Vec3 {
        Self::position_in(self.body, SceneManager::physics())
    }

    pub fn grounded(&self) -> bool {
        self.grounded
    }

    pub(crate) fn position_in(body: RigidBodyHandle, physics: &ScenePhysics) -> Vec3 {
        let translation = physics.sets.rigid_bodies[body].translation();
        Vec3::new(translation.x, translation.y, translation.z)
    }

    pub(crate) fn eye(&self, physics: &ScenePhysics) -> Vec3 {
        Self::position_in(self.body, physics) + Vec3::Y * self.eye_height
    }

    /// The walk the held keys ask for, unit length or zero, and whether
    /// a jump is asked for.
    fn wish(&self) -> (Vec3, bool) {
        if !self.keyboard {
            return (Vec3::ZERO, false);
        }
        let axis = |negative: [KeyCode; 2], positive: [KeyCode; 2]| {
            let held = |keys: [KeyCode; 2]| f32::from(u8::from(keys.iter().any(|key| Keys::held(*key))));
            held(positive) - held(negative)
        };
        let forward = axis(
            [KeyCode::KeyS, KeyCode::ArrowDown],
            [KeyCode::KeyW, KeyCode::ArrowUp],
        );
        let right = axis(
            [KeyCode::KeyA, KeyCode::ArrowLeft],
            [KeyCode::KeyD, KeyCode::ArrowRight],
        );
        let wish = (self.forward() * forward + self.right() * right).normalize_or_zero();
        (wish, Keys::held(KeyCode::Space))
    }

    /// One physics step: walk, fall or jump, slide along what is hit,
    /// push what can move, and hand the result to the kinematic body
    /// the step then applies.
    pub(crate) fn step(&mut self, physics: &mut ScenePhysics, dt: f32) {
        if self.mouse {
            let motion = Cursor::take_motion();
            self.look(motion.x * self.look_speed, -motion.y * self.look_speed);
        }

        let (wish, jump) = self.wish();

        if self.grounded {
            self.vertical = if jump { self.jump_speed } else { 0.0 };
        } else {
            self.vertical += physics.gravity.y * dt;
        }

        let desired = wish * self.speed * dt + Vec3::Y * self.vertical * dt;
        let filter = QueryFilter::default().exclude_rigid_body(self.body);
        let shape = physics.sets.colliders[self.collider].shared_shape().clone();

        let mut collisions = vec![];
        let movement = {
            let queries = physics.broad_phase.as_query_pipeline(
                physics.narrow_phase.query_dispatcher(),
                &physics.sets.rigid_bodies,
                &physics.sets.colliders,
                filter,
            );
            let pose = *physics.sets.colliders[self.collider].position();
            self.controller.move_shape(dt, &queries, &*shape, &pose, desired, |collision| {
                collisions.push(collision);
            })
        };

        self.grounded = movement.grounded;
        if movement.grounded && self.vertical < 0.0 {
            self.vertical = 0.0;
        }

        let mut queries = physics.broad_phase.as_query_pipeline_mut(
            physics.narrow_phase.query_dispatcher(),
            &mut physics.sets.rigid_bodies,
            &mut physics.sets.colliders,
            filter,
        );
        self.controller
            .solve_character_collision_impulses(dt, &mut queries, &*shape, self.mass, &collisions);

        let body = &mut physics.sets.rigid_bodies[self.body];
        let next = body.translation() + movement.translation;
        body.set_next_kinematic_translation(next);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn direction_follows_yaw_and_pitch() {
        let mut player = Player {
            body:       RigidBodyHandle::invalid(),
            collider:   ColliderHandle::invalid(),
            controller: KinematicCharacterController::default(),
            yaw:        0.0,
            pitch:      0.0,
            speed:      4.0,
            jump_speed: 5.0,
            eye_height: 0.7,
            mass:       70.0,
            keyboard:   false,
            mouse:      false,
            look_speed: 0.002,
            vertical:   0.0,
            grounded:   false,
        };
        assert!((player.direction() - Vec3::NEG_Z).length() < 1e-6);
        player.look(FRAC_PI_2, 0.0);
        assert!((player.direction() - Vec3::X).length() < 1e-6);
        player.look(0.0, 10.0);
        assert!(player.direction().y < 1.0 && player.direction().y > 0.99);
        assert!((player.right() - Vec3::Z).length() < 1e-6);
    }
}
