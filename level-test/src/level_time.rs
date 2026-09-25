use std::{thread::sleep, time::Duration};

use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    gm::{Clock, Shape, color::RED},
    level::{Banner, LevelCreation, LevelManager, LevelSetup, LevelTest, Sprite, SpriteTemplates, level},
    refs::Weak,
    time::Instant,
    ui_test::{checkpoint, step_frames},
};

const SPEED: f32 = 10.0;

/// A box running right at ten units a second, a tenth of the screen.
/// The level must run on the real clock, not once per frame: the headless
/// loop draws frames far faster than any screen and a 120 Hz screen twice
/// as fast as a 60 Hz one, and the box must cover the same ground on all.
#[level]
#[derive(Default)]
struct LevelTime {
    runner: Weak<Banner>,
    x:      f32,
    steps:  usize,
}

impl LevelSetup for LevelTime {
    fn setup(&mut self) {
        self.runner = self.make_sprite::<Banner>(Shape::rect(2, 2), (-25, 0));
        self.runner.set_color(RED);
    }

    fn update(&mut self, dt: f32) {
        self.steps += 1;
        self.x = (self.x + dt * SPEED) % 50.0;
        let x = self.x;
        self.runner.set_x(-25.0 + x);
    }
}

impl LevelTest for LevelTime {
    fn perform_test(level: Weak<Self>) -> Result<()> {
        let (start, wall) = from_main(|| (LevelManager::time(), Instant::now()));
        sleep(Duration::from_millis(600));
        let (end, real) = from_main(move || (LevelManager::time(), wall.elapsed().as_secs_f64()));
        let simulated = end - start;
        ensure!(
            (simulated - real).abs() < 0.1,
            "the level ran {simulated:.3} s in {real:.3} s of real time"
        );
        checkpoint("the box runs a tenth of the screen a second")?;

        // On the stepped clock a frame is a 60th of a second, two steps
        // of the default 120th.
        from_main(Clock::enter_stepped);
        let before = from_main(move || level.steps);
        step_frames(30);
        let steps = from_main(move || level.steps) - before;
        ensure!(
            (59..=61).contains(&steps),
            "30 frames at 60 Hz ran {steps} steps, not 60"
        );

        from_main(|| LevelManager::set_step(1.0 / 30.0));
        let before = from_main(move || level.steps);
        step_frames(30);
        let steps = from_main(move || level.steps) - before;
        from_main(|| LevelManager::set_step(LevelManager::DEFAULT_STEP));
        from_main(Clock::exit_stepped);
        ensure!(
            (14..=16).contains(&steps),
            "30 frames at a 30th step ran {steps} steps, not 15"
        );

        Ok(())
    }
}
