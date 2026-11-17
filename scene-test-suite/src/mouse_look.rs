use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::{
        LossyConvert,
        volume::{Shape3, Vec3},
    },
    refs::Weak,
    scene::{Light, NodeTemplates, SceneCreation, SceneSetup, SceneTest, Wall, scene},
    ui::{Color, Cursor, NamedKey},
    ui_test::{capture_screenshot, checkpoint, inject_mouse_motion, inject_named_key},
};

/// Frames for the player to settle onto the floor.
const SETTLE_FRAMES: usize = 20;
/// A mouse moves a little every frame, so a turn arrives as a stream of
/// small motions: this many, one per frame, adding up to `MOTION`, a
/// quarter turn to the right and a little down at the default look
/// speed of 0.002 radians per unit.
const STEPS: usize = 60;
const MOTION: (f32, f32) = (785.0, 150.0);

/// A first person player between a blue post ahead and a red crate to
/// its right turns with the captured mouse: a stream of small motions to
/// the right and down sweeps the view off the post, past the empty floor
/// halfway, onto the crate, the yaw and the pitch add up to the whole
/// motion by the look speed and the camera follows the eyes. With the
/// mouse free, before the capture and after Escape, the same stream
/// turns nothing.
#[scene]
#[derive(Default)]
struct MouseLook {}

impl SceneSetup for MouseLook {
    fn needs_physics(&self) -> bool {
        true
    }

    fn setup(&mut self) {
        self.lights
            .push(Light::point(Vec3::new(0.0, 5.0, 0.0)).intensity(6.0).range(16.0));

        self.make_node::<Wall>(Shape3::Plane(30.0), Vec3::ZERO)
            .set_color(Color::hex("#8d9aa5"))
            .set_roughness(0.9);

        self.make_node::<Wall>(Shape3::cuboid(1.0, 4.0, 1.0), Vec3::new(0.0, 2.0, -5.0))
            .set_color(Color::hex("#3070d0"))
            .set_roughness(0.6);

        self.make_node::<Wall>(Shape3::cube(2.0), Vec3::new(5.0, 1.0, 0.0))
            .set_color(Color::hex("#d04030"))
            .set_roughness(0.6);

        self.add_player(Vec3::new(0.0, 1.0, 0.0));
    }
}

/// One frame of the stream, a `STEPS`th of the whole motion.
fn step() -> (f32, f32) {
    let steps: f32 = STEPS.lossy_convert();
    (MOTION.0 / steps, MOTION.1 / steps)
}

/// Streams `steps` frames of mouse motion, one motion per frame.
fn sweep(steps: usize) {
    for _ in 0..steps {
        inject_mouse_motion(step());
        wait_for_next_frame();
    }
    // The loop runs free, the last motion lands within one more frame.
    wait_for_next_frame();
}

impl SceneTest for MouseLook {
    fn perform_test(scene: Weak<Self>) -> Result<()> {
        let look = move || {
            from_main(move || {
                let player = scene.player.as_ref().expect("the scene has a player");
                (player.yaw, player.pitch, player.look_speed)
            })
        };
        // The stream adds up in floats, so the sum is a hair off.
        let close = |a: f32, b: f32| (a - b).abs() < 1e-4;

        for _ in 0..SETTLE_FRAMES {
            wait_for_next_frame();
        }
        sweep(STEPS);
        let (yaw, pitch, speed) = look();
        ensure!(
            yaw == 0.0 && pitch == 0.0,
            "a free mouse turned the player to {yaw} {pitch}"
        );
        checkpoint("a free mouse moved, the player still looks at the post ahead")?;

        from_main(Cursor::capture);
        ensure!(from_main(Cursor::captured), "the mouse is not captured");

        sweep(STEPS / 2);
        let (yaw, pitch, _) = look();
        let (half_yaw, half_pitch) = (MOTION.0 * speed / 2.0, -MOTION.1 * speed / 2.0);
        ensure!(
            close(yaw, half_yaw) && close(pitch, half_pitch),
            "half the stream turned the player to {yaw} {pitch}, wanted {half_yaw} {half_pitch}"
        );
        checkpoint("half the stream of the captured mouse, the player turned half way")?;

        sweep(STEPS / 2);
        let (yaw, pitch, _) = look();
        let (wanted_yaw, wanted_pitch) = (MOTION.0 * speed, -MOTION.1 * speed);
        ensure!(
            close(yaw, wanted_yaw) && close(pitch, wanted_pitch),
            "the stream turned the player to {yaw} {pitch}, wanted {wanted_yaw} {wanted_pitch}"
        );

        let (ahead, direction) = from_main(move || {
            let player = scene.player.as_ref().expect("the scene has a player");
            (scene.camera.target - scene.camera.position, player.direction())
        });
        ensure!(
            (ahead - direction).length() < 1e-4,
            "the camera looks along {ahead:?}, the player along {direction:?}"
        );
        checkpoint("the whole stream, the player looks at the crate on the right")?;

        inject_named_key(NamedKey::Escape);
        ensure!(!from_main(Cursor::captured), "Escape did not free the mouse");

        sweep(STEPS);
        let (still_yaw, still_pitch, _) = look();
        ensure!(
            close(still_yaw, yaw) && close(still_pitch, pitch),
            "a freed mouse turned the player to {still_yaw} {still_pitch}"
        );
        capture_screenshot()?;
        checkpoint("Escape freed the mouse, the same stream left the player on the crate")
    }
}
