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
    scene::{SceneManager, ThirdPerson, scene::ScenePhysics},
    ui::{Cursor, Keys},
    window::KeyCode,
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
            mass: 70.0,
            keyboard: true,
            walk_keys: WalkKeys::WasdAndArrows,
            mouse: true,
            look_speed: 0.002,
            third_person: None,
            motion: Motion::Walk,
            steer: None,
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

    /// Level and to the right of where the player looks, unit length.
    pub fn right(&self) -> Vec3 {
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

    /// Puts the capsule's center at `position` at once, with no fall
    /// left, the way a game respawns a player that fell off the world.
    pub fn teleport(&mut self, position: impl Into<Vec3>) {
        let position = position.into();
        let body = &mut SceneManager::physics().sets.rigid_bodies[self.body];
        body.set_translation(position, true);
        body.set_next_kinematic_translation(position);
        self.vertical = 0.0;
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
        if let Some(steer) = self.steer {
            let steer = if self.motion == Motion::Fly {
                steer
            } else {
                Vec3::new(steer.x, 0.0, steer.z)
            };
            return (steer.normalize_or_zero(), false);
        }
        if !self.keyboard {
            return (Vec3::ZERO, false);
        }
        let arrows = self.walk_keys == WalkKeys::WasdAndArrows;
        let axis = |negative: [KeyCode; 2], positive: [KeyCode; 2]| {
            // The second key of each pair is the arrow.
            let held = |keys: [KeyCode; 2]| {
                let held = Keys::held(keys[0]) || (arrows && Keys::held(keys[1]));
                f32::from(u8::from(held))
            };
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
        if self.motion == Motion::Fly {
            let up = Self::fly_up(
                Keys::held(KeyCode::Space),
                Keys::held(KeyCode::ControlLeft) || Keys::held(KeyCode::ControlRight),
            );
            let wish = self.direction() * forward + self.right() * right + Vec3::Y * up;
            return (wish.normalize_or_zero(), false);
        }
        let wish = (self.forward() * forward + self.right() * right).normalize_or_zero();
        (wish, Keys::held(KeyCode::Space))
    }

    /// How a flying player climbs, 1 up, -1 down, 0 with both keys or none.
    fn fly_up(space: bool, ctrl: bool) -> f32 {
        f32::from(u8::from(space)) - f32::from(u8::from(ctrl))
    }

    /// One physics step from the mouse and the held keys, see `move_by`.
    pub(crate) fn step(&mut self, physics: &mut ScenePhysics, dt: f32) {
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

        if flying {
            self.vertical = 0.0;
        } else if self.grounded {
            self.vertical = if jump { self.jump_speed } else { 0.0 };
        } else {
            self.vertical += physics.gravity.y * dt;
        }
        // A rising player is off the ground. Snapping it back, or reading it
        // as grounded while it is still within the controller's offset of
        // the ground, ends every jump in its first step.
        let rising = self.vertical > 0.0;
        if rising {
            self.controller.snap_to_ground = None;
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
        body.set_next_kinematic_translation(next);
    }
}

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use super::*;
    use crate::scene::{ColliderShape, Heightfield};

    fn test_player() -> Player {
        Player {
            body:         RigidBodyHandle::invalid(),
            collider:     ColliderHandle::invalid(),
            controller:   KinematicCharacterController::default(),
            yaw:          0.0,
            pitch:        0.0,
            speed:        4.0,
            jump_speed:   5.0,
            eye_height:   0.7,
            mass:         70.0,
            keyboard:     false,
            walk_keys:    WalkKeys::WasdAndArrows,
            mouse:        false,
            look_speed:   0.002,
            third_person: None,
            motion:       Motion::Walk,
            steer:        None,
            vertical:     0.0,
            grounded:     false,
        }
    }

    #[test]
    fn a_steer_moves_the_player_in_place_of_the_keys() {
        let mut player = test_player();
        assert_eq!(player.wish(), (Vec3::ZERO, false));

        player.steer = Some(Vec3::new(3.0, 4.0, 0.0));
        let (wish, jump) = player.wish();
        assert!((wish - Vec3::X).length() < 1e-6, "a walk is level: {wish}");
        assert!(!jump);

        player.motion = Motion::Fly;
        let (wish, _) = player.wish();
        assert!((wish - Vec3::new(0.6, 0.8, 0.0)).length() < 1e-6, "{wish}");
    }

    /// A player at `position` over flat ground at 0 that rises into a
    /// wall 100 high from `z = -50` on.
    fn over_ground(position: Vec3) -> (ScenePhysics, Player) {
        let mut physics = ScenePhysics::default();
        let field = Heightfield::from_fn(201, 201, 400.0, 400.0, |_, z| if z < -50.0 { 100.0 } else { 0.0 });
        let ground = ColliderShape::Heightfield(Arc::new(field)).builder(1.0).build();
        physics.sets.colliders.insert(ground);
        let player = Player::make(&mut physics, position, 0.4, 1.8);
        physics.update_physics(&[], DT);
        (physics, player)
    }

    /// Steps of 1/60 s, so 60 is a second.
    fn run(physics: &mut ScenePhysics, player: &mut Player, steps: usize, wish: Vec3) {
        for _ in 0..steps {
            player.move_by(physics, DT, wish, false);
            physics.update_physics(&[], DT);
        }
    }

    fn position(physics: &ScenePhysics, player: &Player) -> Vec3 {
        Player::position_in(player.body, physics)
    }

    const DT: f32 = 1.0 / 60.0;

    /// The steps the scene really takes, a frame of 1/60 s in
    /// `PHYSICS_SUBSTEPS` parts, and how many make a second.
    const SUBSTEP: f32 = DT / 4.0;
    const SUBSTEPS_A_SECOND: usize = 240;

    #[test]
    fn a_walking_player_keeps_its_speed_in_substeps() {
        assert_eq!(
            crate::scene::SceneBase::PHYSICS_SUBSTEPS,
            4,
            "the substep of these tests"
        );
        let (mut physics, mut player) = over_ground(Vec3::new(0.0, 0.95, 100.0));
        for _ in 0..240 {
            player.move_by(&mut physics, SUBSTEP, Vec3::ZERO, false);
            physics.update_physics(&[], SUBSTEP);
        }
        let start = position(&physics, &player);
        for _ in 0..SUBSTEPS_A_SECOND {
            player.move_by(&mut physics, SUBSTEP, Vec3::NEG_Z, false);
            physics.update_physics(&[], SUBSTEP);
        }
        let walked = start.z - position(&physics, &player).z;
        assert!(
            (walked - player.speed).abs() < 0.2,
            "walked {walked} m in 1 s at {} m/s",
            player.speed
        );
    }

    /// Ground at 0 with bumps of a few centimeters every half meter, like
    /// the land of a game.
    fn over_bumps(position: Vec3) -> (ScenePhysics, Player) {
        let mut physics = ScenePhysics::default();
        let field = Heightfield::from_fn(801, 801, 400.0, 400.0, |x, z| {
            0.04 * (x * 3.1).sin() * (z * 2.7).cos() + 0.02 * (x * 7.3 + z * 5.9).sin()
        });
        let ground = ColliderShape::Heightfield(Arc::new(field)).builder(1.0).build();
        physics.sets.colliders.insert(ground);
        let player = Player::make(&mut physics, position, 0.4, 1.8);
        physics.update_physics(&[], DT);
        (physics, player)
    }

    #[test]
    fn a_walking_player_keeps_its_speed_over_bumps() {
        let (mut physics, mut player) = over_bumps(Vec3::new(0.0, 0.95, 100.0));
        for _ in 0..240 {
            player.move_by(&mut physics, SUBSTEP, Vec3::ZERO, false);
            physics.update_physics(&[], SUBSTEP);
        }
        let start = position(&physics, &player);
        for _ in 0..SUBSTEPS_A_SECOND {
            player.move_by(&mut physics, SUBSTEP, Vec3::NEG_Z, false);
            physics.update_physics(&[], SUBSTEP);
        }
        let walked = start.z - position(&physics, &player).z;
        assert!(
            (walked - player.speed).abs() < 0.3,
            "walked {walked} m in 1 s at {} m/s",
            player.speed
        );
    }

    #[test]
    fn a_jump_lands_in_the_time_its_speed_gives() {
        let (mut physics, mut player) = over_ground(Vec3::new(0.0, 0.95, 100.0));
        for _ in 0..240 {
            player.move_by(&mut physics, SUBSTEP, Vec3::ZERO, false);
            physics.update_physics(&[], SUBSTEP);
        }
        player.move_by(&mut physics, SUBSTEP, Vec3::ZERO, true);
        physics.update_physics(&[], SUBSTEP);
        let mut steps: u16 = 1;
        while !player.grounded && steps < 2000 {
            player.move_by(&mut physics, SUBSTEP, Vec3::ZERO, false);
            physics.update_physics(&[], SUBSTEP);
            steps += 1;
        }
        let air = f32::from(steps) * SUBSTEP;
        let expected = 2.0 * player.jump_speed / -physics.gravity.y;
        assert!(
            (air - expected).abs() < 0.1,
            "in the air {air} s, a jump of {} m/s takes {expected} s",
            player.jump_speed
        );
    }

    #[test]
    fn a_flying_player_rises_where_it_looks() {
        let (mut physics, mut player) = over_ground(Vec3::new(0.0, 10.0, 100.0));
        player.motion = Motion::Fly;
        player.speed = 10.0;
        player.look(0.0, 0.5);
        let wish = player.direction();
        run(&mut physics, &mut player, 60, wish);
        let at = position(&physics, &player);
        assert!(
            (at - Vec3::new(0.0, 10.0, 100.0) - wish * 10.0).length() < 0.1,
            "{at}"
        );
    }

    #[test]
    fn space_lifts_a_flying_player_and_ctrl_lowers_it() {
        assert_eq!(Player::fly_up(true, false), 1.0);
        assert_eq!(Player::fly_up(false, true), -1.0);
        assert_eq!(Player::fly_up(true, true), 0.0);
        let (mut physics, mut player) = over_ground(Vec3::new(0.0, 10.0, 100.0));
        player.motion = Motion::Fly;
        player.speed = 10.0;
        run(
            &mut physics,
            &mut player,
            30,
            Vec3::Y * Player::fly_up(true, false),
        );
        assert!((position(&physics, &player).y - 15.0).abs() < 0.1);
        run(
            &mut physics,
            &mut player,
            60,
            Vec3::Y * Player::fly_up(false, true),
        );
        assert!((position(&physics, &player).y - 5.0).abs() < 0.1);
    }

    #[test]
    fn a_flying_player_with_no_keys_holds_its_height() {
        let (mut physics, mut player) = over_ground(Vec3::new(0.0, 10.0, 100.0));
        player.motion = Motion::Fly;
        run(&mut physics, &mut player, 120, Vec3::ZERO);
        assert!((position(&physics, &player).y - 10.0).abs() < 1e-3);
    }

    #[test]
    fn a_fast_flying_player_stops_at_a_hill() {
        let (mut physics, mut player) = over_ground(Vec3::new(0.0, 10.0, 100.0));
        player.motion = Motion::Fly;
        player.speed = 300.0;
        run(&mut physics, &mut player, 120, Vec3::NEG_Z);
        let at = position(&physics, &player);
        assert!(at.z > -50.0 && at.z < -49.0, "{at}");
        assert!((at.y - 10.0).abs() < 0.5, "{at}");
    }

    #[test]
    fn a_player_that_stops_flying_falls() {
        let (mut physics, mut player) = over_ground(Vec3::new(0.0, 10.0, 100.0));
        player.motion = Motion::Fly;
        run(&mut physics, &mut player, 30, Vec3::ZERO);
        player.motion = Motion::Walk;
        run(&mut physics, &mut player, 180, Vec3::ZERO);
        let at = position(&physics, &player);
        assert!(at.y < 1.0 && player.grounded(), "{at}");
    }

    #[test]
    fn direction_follows_yaw_and_pitch() {
        let mut player = test_player();
        assert!((player.direction() - Vec3::NEG_Z).length() < 1e-6);
        player.look(FRAC_PI_2, 0.0);
        assert!((player.direction() - Vec3::X).length() < 1e-6);
        player.look(0.0, 10.0);
        assert!(player.direction().y < 1.0 && player.direction().y > 0.99);
        assert!((player.right() - Vec3::Z).length() < 1e-6);
    }

    #[test]
    fn the_arrows_walk_only_while_allowed() {
        let player = Player {
            keyboard: true,
            ..test_player()
        };
        Keys::set(KeyCode::ArrowUp, true);
        let walks = player.wish().0.length() > 0.5;
        let blocked = Player {
            walk_keys: WalkKeys::WasdOnly,
            ..player
        }
        .wish()
        .0;
        Keys::set(KeyCode::ArrowUp, false);
        assert!(walks);
        assert_eq!(blocked, Vec3::ZERO);
    }
}
