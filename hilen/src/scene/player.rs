use std::f32::consts::FRAC_PI_2;

use rapier3d::{
    control::{CharacterAutostep, CharacterLength, KinematicCharacterController},
    dynamics::RigidBodyHandle,
    geometry::ColliderHandle,
    pipeline::QueryFilter,
    prelude::{ColliderBuilder, Pose3, RigidBodyBuilder},
};

use crate::{
    gm::volume::{Quat, Vec3},
    scene::{SceneManager, ThirdPerson, WorldMove, scene::ScenePhysics},
    ui::{Code, Cursor, Keys},
};

/// Looking straight up or down flips the view, so the pitch stops just
/// short of it.
const PITCH_LIMIT: f32 = FRAC_PI_2 - 0.05;

/// The highest ledge a walking player steps up on.
const AUTOSTEP: Option<CharacterAutostep> = Some(CharacterAutostep {
    max_height:             CharacterLength::Absolute(0.35),
    min_width:              CharacterLength::Absolute(0.2),
    include_dynamic_bodies: true,
});
/// How far down a walking player sticks to the ground it walks over.
const SNAP_TO_GROUND: Option<CharacterLength> = Some(CharacterLength::Absolute(0.2));

/// How a player moves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Motion {
    /// On the ground under gravity, with a jump and small steps.
    Walk,
    /// With no gravity, `w` and `s` along the full `direction`, the pitch
    /// included, space straight up and ctrl straight down. It still
    /// slides along what it hits. Back to `Walk`, it falls from where it
    /// is.
    Fly,
    /// Out of the physics and still. The capsule touches nothing, no key,
    /// mouse, drift or fall moves the player, and the scene leaves the
    /// camera to the game, a hero that sits down and looks out of its own
    /// head. Back to `Walk`, the capsule collides again and the player walks
    /// on from where it was.
    Parked,
}

/// The keys that walk a player.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WalkKeys {
    WasdAndArrows,
    WasdOnly,
}

/// A player. A capsule the rapier character controller walks over the
/// scene's colliders, with gravity, small steps, a jump and a push on the
/// bodies it walks into. The scene camera looks out of its eyes, or with
/// `third_person` over its shoulder. `w` `a` `s` `d` or the arrows walk,
/// space jumps, the captured mouse or `look` turns. With `Motion::Fly` it
/// flies where it looks, space lifts it and ctrl lowers it.
pub struct Player {
    body:       RigidBodyHandle,
    collider:   ColliderHandle,
    controller: KinematicCharacterController,

    /// Radians around the up axis, 0 looks down `-z`, positive turns
    /// right.
    pub yaw:          f32,
    /// Radians up from level.
    pub pitch:        f32,
    /// Units per second.
    pub speed:        f32,
    /// The upward speed a jump starts with.
    pub jump_speed:   f32,
    /// The eye above the capsule's center.
    pub eye_height:   f32,
    /// Where the first person camera sits off the eye, in the player's own
    /// turn: x to its right, y up, z behind it. A game whose body plays
    /// clips sets it every frame from the head of the body, so the camera
    /// moves with the head. The capsule, the grass and `third_person` keep
    /// the eye. Zero by default.
    pub eye_offset:   Vec3,
    /// What the player weighs when it pushes a body.
    pub mass:         f32,
    /// Reads the keyboard every step. Off for a scene that moves the
    /// player itself.
    pub keyboard:     bool,
    /// Which keys walk. A game that gives the arrows another job walks on
    /// `w` `a` `s` `d` only.
    pub walk_keys:    WalkKeys,
    /// Turns with the captured mouse every step, see `Cursor`. Off for
    /// a scene that turns the player itself.
    pub mouse:        bool,
    /// Radians of turn per unit of mouse motion.
    pub look_speed:   f32,
    /// Where the camera sits: out of the eyes while `None`, else behind
    /// the player over its shoulder.
    pub third_person: Option<ThirdPerson>,
    /// Walks or flies.
    pub motion:       Motion,
    /// A move asked for by code in place of the keys, an autopilot or a
    /// player a test drives. It goes the same way as a held key does, at
    /// `speed` along its direction and with every collision, a walking
    /// player takes it level. The tools once held keys through the
    /// inspector for a test flight and got a different flight every run.
    pub steer:        Option<Vec3>,
    /// An outside speed in units per second, wind, a conveyor or a river.
    /// It adds to the keys and to `steer`, with every collision, and stays
    /// until the game changes it.
    pub drift:        Vec3,

    frame:    Quat,
    vertical: f32,
    grounded: bool,
}

impl Player {
    /// The capsule the player collides as, what scene queries leave out.
    pub(crate) fn collider_handle(&self) -> ColliderHandle {
        self.collider
    }

    pub(crate) fn make(physics: &mut ScenePhysics, position: Vec3, radius: f32, height: f32) -> Self {
        let half_height = (height / 2.0 - radius).max(0.0);
        let body = RigidBodyBuilder::kinematic_position_based().translation(position).build();
        let collider = ColliderBuilder::capsule_y(half_height, radius).build();
        let (body, collider) = physics.sets.insert(body, collider);

        let controller = KinematicCharacterController {
            offset: CharacterLength::Absolute(0.02),
            autostep: AUTOSTEP,
            snap_to_ground: SNAP_TO_GROUND,
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
            eye_offset: Vec3::ZERO,
            mass: 70.0,
            keyboard: true,
            walk_keys: WalkKeys::WasdAndArrows,
            mouse: true,
            look_speed: 0.002,
            third_person: None,
            motion: Motion::Walk,
            steer: None,
            drift: Vec3::ZERO,
            frame: Quat::IDENTITY,
            vertical: 0.0,
            grounded: false,
        }
    }

    /// Turn by `yaw` and `pitch` radians.
    pub fn look(&mut self, yaw: f32, pitch: f32) {
        self.yaw += yaw;
        self.pitch = (self.pitch + pitch).clamp(-PITCH_LIMIT, PITCH_LIMIT);
    }

    /// Where the player's own y points, against the pull of gravity, see
    /// `frame`. Y while gravity is the scene's uniform 9.81 down.
    pub fn up(&self) -> Vec3 {
        self.frame * Vec3::Y
    }

    /// The turn from the world's axes to the player's own, which `yaw`
    /// and `pitch` are measured in. Every step it turns by the least
    /// turn that puts its y against the pull at the player, so a walk
    /// round a planet keeps its heading, the way a compass needle is
    /// carried along. With no pull, in space, it keeps its turn.
    pub fn frame(&self) -> Quat {
        self.frame
    }

    /// Where the player looks, unit length.
    pub fn direction(&self) -> Vec3 {
        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();
        let (sin_pitch, cos_pitch) = self.pitch.sin_cos();
        self.frame * Vec3::new(sin_yaw * cos_pitch, sin_pitch, -cos_yaw * cos_pitch)
    }

    /// Level ahead of where the player looks, unit length.
    pub fn forward(&self) -> Vec3 {
        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();
        self.frame * Vec3::new(sin_yaw, 0.0, -cos_yaw)
    }

    /// Level and to the right of where the player looks, unit length.
    pub fn right(&self) -> Vec3 {
        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();
        self.frame * Vec3::new(cos_yaw, 0.0, sin_yaw)
    }

    /// Turns the frame by the least turn that puts its y against `pull`.
    fn stand_against(&mut self, pull: Vec3) {
        // Below this the pull has no direction worth following.
        if pull.length_squared() < 1e-8 {
            return;
        }
        let turn = Quat::from_rotation_arc(self.up(), -pull.normalize());
        self.frame = (turn * self.frame).normalize();
    }

    /// The center of the capsule.
    pub fn position(&self) -> Vec3 {
        Self::position_in(self.body, SceneManager::physics())
    }

    pub fn grounded(&self) -> bool {
        self.grounded
    }

    /// Puts the capsule's center at `position` at once, with no fall
    /// left, the way a game respawns a player that fell off the world.
    pub fn teleport(&mut self, position: impl Into<Vec3>) {
        let position = position.into();
        let body = &mut SceneManager::physics().sets.rigid_bodies[self.body];
        body.set_translation(position, true);
        body.set_next_kinematic_translation(position);
        self.vertical = 0.0;
    }

    /// Moves the capsule with the rest of the scene and turns its frame
    /// along, so the view stays on what it looked at, see
    /// `SceneBase::move_world`. The fall or the jump goes on.
    pub(crate) fn move_world(&mut self, physics: &mut ScenePhysics, world: WorldMove) {
        self.frame = world.rotation(self.frame);
        let pose = Pose3::from_parts(world.point(Self::position_in(self.body, physics)), self.frame);
        let body = &mut physics.sets.rigid_bodies[self.body];
        body.set_position(pose, false);
        body.set_next_kinematic_position(pose);
        self.steer = self.steer.map(|steer| world.direction(steer));
        self.drift = world.direction(self.drift);
    }

    pub(crate) fn position_in(body: RigidBodyHandle, physics: &ScenePhysics) -> Vec3 {
        let translation = physics.sets.rigid_bodies[body].translation();
        Vec3::new(translation.x, translation.y, translation.z)
    }

    pub(crate) fn eye(&self, physics: &ScenePhysics) -> Vec3 {
        Self::position_in(self.body, physics) + self.up() * self.eye_height
    }

    /// The walk the held keys ask for, unit length or zero, and whether
    /// a jump is asked for.
    fn wish(&self) -> (Vec3, bool) {
        if let Some(steer) = self.steer {
            let steer = if self.motion == Motion::Fly {
                steer
            } else {
                steer - self.up() * steer.dot(self.up())
            };
            return (steer.normalize_or_zero(), false);
        }
        if !self.keyboard {
            return (Vec3::ZERO, false);
        }
        let arrows = self.walk_keys == WalkKeys::WasdAndArrows;
        let axis = |negative: [Code; 2], positive: [Code; 2]| {
            // The second key of each pair is the arrow.
            let held = |keys: [Code; 2]| {
                let held = Keys::held(keys[0]) || (arrows && Keys::held(keys[1]));
                f32::from(u8::from(held))
            };
            held(positive) - held(negative)
        };
        let forward = axis([Code::KeyS, Code::ArrowDown], [Code::KeyW, Code::ArrowUp]);
        let right = axis([Code::KeyA, Code::ArrowLeft], [Code::KeyD, Code::ArrowRight]);
        if self.motion == Motion::Fly {
            let up = Self::fly_up(
                Keys::held(Code::Space),
                Keys::held(Code::ControlLeft) || Keys::held(Code::ControlRight),
            );
            let wish = self.direction() * forward + self.right() * right + self.up() * up;
            return (wish.normalize_or_zero(), false);
        }
        let wish = (self.forward() * forward + self.right() * right).normalize_or_zero();
        (wish, Keys::held(Code::Space))
    }

    /// How a flying player climbs, 1 up, -1 down, 0 with both keys or none.
    fn fly_up(space: bool, ctrl: bool) -> f32 {
        f32::from(u8::from(space)) - f32::from(u8::from(ctrl))
    }

    /// One physics step from the mouse and the held keys, see `move_by`.
    pub(crate) fn step(&mut self, physics: &mut ScenePhysics, dt: f32) {
        let parked = self.motion == Motion::Parked;
        let capsule = &mut physics.sets.colliders[self.collider];
        if capsule.is_enabled() == parked {
            capsule.set_enabled(!parked);
        }
        if parked {
            // A fall that was going on at the park would go on after it.
            self.vertical = 0.0;
            return;
        }
        if self.mouse {
            let motion = Cursor::take_motion();
            self.look(motion.x * self.look_speed, -motion.y * self.look_speed);
        }

        let (wish, jump) = self.wish();
        self.move_by(physics, dt, wish, jump);
    }

    /// Walk toward `wish`, fall or jump, or fly, slide along what is hit,
    /// push what can move, and hand the result to the kinematic body the
    /// physics step then applies.
    fn move_by(&mut self, physics: &mut ScenePhysics, dt: f32, wish: Vec3, jump: bool) {
        // Snapping to the ground would pull a player flying low down onto it.
        let flying = self.motion == Motion::Fly;
        self.controller.autostep = if flying { None } else { AUTOSTEP };
        self.controller.snap_to_ground = if flying { None } else { SNAP_TO_GROUND };

        let pull = physics.gravity_at(Self::position_in(self.body, physics));
        self.stand_against(pull);
        let up = self.up();
        self.controller.up = up;
        if flying {
            self.vertical = 0.0;
        } else if self.grounded {
            self.vertical = if jump { self.jump_speed } else { 0.0 };
        } else {
            self.vertical += pull.dot(up) * dt;
        }
        // A rising player is off the ground. Snapping it back, or reading it
        // as grounded while it is still within the controller's offset of
        // the ground, ends every jump in its first step.
        let rising = self.vertical > 0.0;
        if rising {
            self.controller.snap_to_ground = None;
        }

        let desired = (wish * self.speed + self.drift + up * self.vertical) * dt;
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
            // The capsule stands along the frame, also in the first step
            // after the frame turned.
            let at = physics.sets.colliders[self.collider].translation();
            let pose = Pose3::from_parts(Vec3::new(at.x, at.y, at.z), self.frame);
            self.controller.move_shape(dt, &queries, &*shape, &pose, desired, |collision| {
                collisions.push(collision);
            })
        };

        self.grounded = movement.grounded && !rising;
        if self.grounded && self.vertical < 0.0 {
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
        body.set_next_kinematic_position(Pose3::from_parts(next, self.frame));
    }
}

#[cfg(test)]
#[path = "player_test.rs"]
mod test;
