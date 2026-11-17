use std::sync::Arc;

use serial_test::serial;

use super::*;
use crate::{
    deps::{hreads::set_current_thread_as_main, refs::Own},
    scene::{ColliderShape, Heightfield, SceneInternal, SceneSetup, scene},
};

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
        eye_offset:   Vec3::ZERO,
        mass:         70.0,
        keyboard:     false,
        walk_keys:    WalkKeys::WasdAndArrows,
        mouse:        false,
        look_speed:   0.002,
        third_person: None,
        motion:       Motion::Walk,
        steer:        None,
        drift:        Vec3::ZERO,
        frame:        Quat::IDENTITY,
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

const PLANET: f32 = 50.0;

/// A ball planet of radius 50 at the origin that pulls 9.81 on its
/// ground, and a player standing on its top with nothing else pulling.
fn on_a_planet() -> (ScenePhysics, Player) {
    let mut physics = ScenePhysics::default();
    physics.gravity = Vec3::ZERO;
    physics.wells = vec![crate::scene::GravityWell::with_surface_gravity(
        Vec3::ZERO,
        PLANET,
        9.81,
    )];
    physics.sets.colliders.insert(ColliderBuilder::ball(PLANET).build());
    let mut player = Player::make(&mut physics, Vec3::new(0.0, PLANET + 0.95, 0.0), 0.4, 1.8);
    player.keyboard = false;
    player.mouse = false;
    physics.update_physics(&[], SUBSTEP);
    (physics, player)
}

/// Walking ahead for 10 s at 4 m/s goes 40 m round the planet, 0.8
/// radians, on its ground the whole way, on 1 great circle, the up
/// turning to stay against the pull.
#[test]
fn a_player_walks_round_a_planet() {
    let (mut physics, mut player) = on_a_planet();
    for _ in 0..SUBSTEPS_A_SECOND {
        player.move_by(&mut physics, SUBSTEP, Vec3::ZERO, false);
        physics.update_physics(&[], SUBSTEP);
    }
    assert!(player.grounded(), "the player stands on the planet");
    for _ in 0..SUBSTEPS_A_SECOND * 10 {
        let ahead = player.forward();
        player.move_by(&mut physics, SUBSTEP, ahead, false);
        physics.update_physics(&[], SUBSTEP);
        let at = position(&physics, &player);
        assert!(
            (at.length() - PLANET - 0.9).abs() < 0.1,
            "the capsule left the ground, its middle {} from the center",
            at.length()
        );
    }
    let at = position(&physics, &player);
    let walked = at.angle_between(Vec3::Y);
    assert!((walked - 0.8).abs() < 0.08, "walked {walked} radians round");
    assert!(at.x.abs() < 0.05, "the walk left its great circle, x {}", at.x);
    assert!(
        player.up().angle_between(at) < 0.01,
        "the up {} is not against the pull",
        player.up()
    );
    assert!(player.forward().dot(player.up()).abs() < 1e-4, "ahead is level");
    assert!(player.grounded());
}

#[test]
fn a_world_move_turns_the_frame_and_keeps_the_view() {
    let (mut physics, mut player) = on_a_planet();
    player.look(0.4, -0.2);
    let (look, up) = (player.direction(), player.up());
    let world = WorldMove {
        turn:  Quat::from_rotation_x(0.7) * Quat::from_rotation_z(-0.3),
        shift: Vec3::new(5.0, -20.0, 8.0),
    };
    player.move_world(&mut physics, world);
    assert!((player.direction() - world.direction(look)).length() < 1e-5);
    assert!((player.up() - world.direction(up)).length() < 1e-5);
}

/// A player 50 above a well whose pull there is 4 falls 2 in 1 second.
#[test]
fn a_player_falls_toward_a_well() {
    let mut physics = ScenePhysics::default();
    physics.gravity = Vec3::ZERO;
    physics.wells = vec![crate::scene::GravityWell::with_surface_gravity(
        Vec3::new(0.0, -10_000.0, 0.0),
        10_000.0,
        4.0,
    )];
    let start = Vec3::new(0.0, 50.0, 0.0);
    let mut player = Player::make(&mut physics, start, 0.4, 1.8);
    for _ in 0..SUBSTEPS_A_SECOND {
        player.move_by(&mut physics, SUBSTEP, Vec3::ZERO, false);
        physics.update_physics(&[], SUBSTEP);
    }
    let fallen = start.y - position(&physics, &player).y;
    assert!((fallen - 2.0).abs() < 0.1, "fell {fallen} in 1 s");
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

/// A settled player on flat ground, away from the wall.
fn settled() -> (ScenePhysics, Player) {
    let (mut physics, mut player) = over_ground(Vec3::new(0.0, 0.95, 100.0));
    for _ in 0..SUBSTEPS_A_SECOND {
        player.move_by(&mut physics, SUBSTEP, Vec3::ZERO, false);
        physics.update_physics(&[], SUBSTEP);
    }
    (physics, player)
}

#[test]
fn a_drift_moves_a_player_with_the_keys_up() {
    let (mut physics, mut player) = settled();
    let start = position(&physics, &player);
    player.drift = Vec3::new(2.0, 0.0, 0.0);
    for _ in 0..SUBSTEPS_A_SECOND {
        player.move_by(&mut physics, SUBSTEP, Vec3::ZERO, false);
        physics.update_physics(&[], SUBSTEP);
    }
    let moved = position(&physics, &player) - start;
    assert!((moved.x - 2.0).abs() < 0.1, "drifted {moved} in 1 s");
    assert!(moved.y.abs() < 0.05, "the drift left the ground: {moved}");
    assert!(player.grounded(), "a drift keeps the feet on the ground");
}

#[test]
fn a_drift_adds_to_a_walk() {
    let (mut physics, mut player) = settled();
    let start = position(&physics, &player);
    player.drift = Vec3::new(2.0, 0.0, 0.0);
    for _ in 0..SUBSTEPS_A_SECOND {
        player.move_by(&mut physics, SUBSTEP, Vec3::NEG_Z, false);
        physics.update_physics(&[], SUBSTEP);
    }
    let moved = position(&physics, &player) - start;
    assert!((moved.x - 2.0).abs() < 0.2, "the drift: {moved}");
    assert!((moved.z + player.speed).abs() < 0.2, "the walk: {moved}");
}

#[test]
fn a_wall_stops_a_drift() {
    let (mut physics, mut player) = over_ground(Vec3::new(0.0, 0.95, -45.0));
    for _ in 0..SUBSTEPS_A_SECOND {
        player.move_by(&mut physics, SUBSTEP, Vec3::ZERO, false);
        physics.update_physics(&[], SUBSTEP);
    }
    player.drift = Vec3::new(0.0, 0.0, -10.0);
    for _ in 0..SUBSTEPS_A_SECOND * 3 {
        player.move_by(&mut physics, SUBSTEP, Vec3::ZERO, false);
        physics.update_physics(&[], SUBSTEP);
    }
    let at = position(&physics, &player);
    assert!(at.z > -50.0 && at.z < -48.0, "{at}");
}

#[test]
fn a_world_move_turns_the_drift() {
    let (mut physics, mut player) = on_a_planet();
    player.drift = Vec3::X;
    let world = WorldMove {
        turn:  Quat::from_rotation_y(0.7),
        shift: Vec3::new(5.0, -20.0, 8.0),
    };
    player.move_world(&mut physics, world);
    assert!((player.drift - world.direction(Vec3::X)).length() < 1e-5);
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
    Keys::set(Code::ArrowUp, true);
    let walks = player.wish().0.length() > 0.5;
    let blocked = Player {
        walk_keys: WalkKeys::WasdOnly,
        ..player
    }
    .wish()
    .0;
    Keys::set(Code::ArrowUp, false);
    assert!(walks);
    assert_eq!(blocked, Vec3::ZERO);
}

/// A settled player that only the code moves, parked after its first step.
fn parked() -> (ScenePhysics, Player) {
    let (mut physics, mut player) = settled();
    player.keyboard = false;
    player.mouse = false;
    player.motion = Motion::Parked;
    player.step(&mut physics, SUBSTEP);
    physics.update_physics(&[], SUBSTEP);
    (physics, player)
}

fn steps(physics: &mut ScenePhysics, player: &mut Player, count: usize) {
    for _ in 0..count {
        player.step(physics, SUBSTEP);
        physics.update_physics(&[], SUBSTEP);
    }
}

#[test]
fn a_parked_player_stays_where_it_is() {
    let (mut physics, mut player) = parked();
    let start = position(&physics, &player);
    player.steer = Some(Vec3::X);
    player.drift = Vec3::new(0.0, 0.0, 2.0);
    steps(&mut physics, &mut player, SUBSTEPS_A_SECOND);
    let moved = position(&physics, &player) - start;
    assert!(moved.length() < 1e-4, "a parked player moved {moved}");

    player.motion = Motion::Walk;
    steps(&mut physics, &mut player, SUBSTEPS_A_SECOND);
    let moved = position(&physics, &player) - start;
    assert!(moved.x > 3.0, "the player walks on after the park: {moved}");
}

#[test]
fn a_parked_player_does_not_fall() {
    let (mut physics, mut player) = over_ground(Vec3::new(0.0, 10.0, 100.0));
    player.keyboard = false;
    player.mouse = false;
    player.motion = Motion::Parked;
    steps(&mut physics, &mut player, SUBSTEPS_A_SECOND);
    let height = position(&physics, &player).y;
    assert!((height - 10.0).abs() < 1e-4, "a parked player fell to {height}");
}

/// How far from the axis of the capsule a floating ball comes in 1 s,
/// thrown at its middle from 1 to the side at 1 a second.
fn thrown_at(parked_player: bool) -> f32 {
    let (mut physics, mut player) = parked();
    player.motion = if parked_player {
        Motion::Parked
    } else {
        Motion::Walk
    };
    steps(&mut physics, &mut player, 1);
    let axis = position(&physics, &player);
    let body = RigidBodyBuilder::dynamic()
        .translation(axis + Vec3::X)
        .linvel(Vec3::NEG_X)
        .gravity_scale(0.0)
        .build();
    let (ball, _) = physics.sets.insert(body, ColliderBuilder::ball(0.1).build());
    steps(&mut physics, &mut player, SUBSTEPS_A_SECOND);
    (Player::position_in(ball, &physics) - axis).length()
}

// The hands of a seated hero work inside the radius of its capsule, and
// the capsule stopped them at its side.
#[test]
fn a_parked_capsule_lets_bodies_through() {
    let through = thrown_at(true);
    assert!(
        through < 0.05,
        "the parked capsule stopped the ball {through} from its axis"
    );
    let stopped = thrown_at(false);
    assert!(
        stopped > 0.45,
        "a solid capsule stops the ball at its side, it came to {stopped}"
    );
}

#[scene]
#[derive(Default)]
struct Seated {}

impl SceneSetup for Seated {
    fn needs_physics(&self) -> bool {
        true
    }
}

#[test]
#[serial]
fn a_parked_player_leaves_the_camera_to_the_game() {
    set_current_thread_as_main();
    let mut scene = Own::new(Seated::default());
    scene.__internal_setup();
    let player = scene.add_player(Vec3::new(0.0, 10.0, 0.0));
    player.keyboard = false;
    player.mouse = false;
    player.motion = Motion::Parked;
    let head = Vec3::new(1.0, 2.0, 3.0);
    scene.camera.position = head;
    scene.__internal_update(DT);
    assert_eq!(
        scene.camera.position, head,
        "the scene moved the camera of a parked player"
    );

    scene.player.as_mut().expect("added").motion = Motion::Walk;
    scene.__internal_update(DT);
    let eyes = scene.camera.position;
    assert!(
        eyes.x.abs() < 1e-3 && eyes.y > 10.0,
        "the camera is back in the eyes: {eyes}"
    );
}

#[test]
#[serial]
fn the_camera_sits_off_the_eyes_by_the_offset_in_the_turn_of_the_player() {
    set_current_thread_as_main();
    let mut scene = Own::new(Seated::default());
    scene.__internal_setup();
    let player = scene.add_player(Vec3::new(0.0, 10.0, 0.0));
    player.keyboard = false;
    player.mouse = false;
    // A flying player with no keys holds its height, so the eyes stay.
    player.motion = Motion::Fly;
    // It looks along +x.
    player.yaw = FRAC_PI_2;
    scene.__internal_update(DT);
    let eyes = scene.camera.position;

    scene.player.as_mut().expect("added").eye_offset = Vec3::new(0.0, 0.1, -0.2);
    scene.__internal_update(DT);
    let moved = scene.camera.position - eyes;
    assert!(
        (moved - Vec3::new(0.2, 0.1, 0.0)).length() < 1e-4,
        "the camera moved by {moved}, 0.2 ahead and 0.1 up"
    );
    let look = scene.camera.target - scene.camera.position;
    assert!(
        (look - Vec3::X).length() < 1e-4,
        "the camera still looks along +x: {look}"
    );
}
