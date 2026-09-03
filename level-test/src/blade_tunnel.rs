use std::f32::consts::PI;

use anyhow::Result;
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::{LossyConvert, Shape},
    level::{Body, LevelCreation, LevelSetup, LevelTest, MovingWall, Sprite, Wall, level},
    refs::Weak,
    ui::Point,
};

const HALF: f32 = 14.0;
const WALL: f32 = 1.2;
const BLADE_LENGTH: f32 = 11.0;
// The demo chamber at its top slider speed, one turn per second.
const TURNS_PER_SECOND: f32 = 1.0;
const BODIES: usize = 20;
const FRAMES: usize = 300;

/// A walled box with a blade spinning at the demo chamber's top speed
/// and a dozen small bodies dropped onto it. Before the physics
/// substeps and CCD a blade jump was wider than a body, so bodies
/// ended up inside the blade or were flung through the wall.
#[level]
#[derive(Default)]
struct BladeTunnel {
    blade:  Option<Weak<MovingWall>>,
    bodies: Vec<Weak<Body>>,
    angle:  f32,
}

impl LevelSetup for BladeTunnel {
    fn needs_physics(&self) -> bool {
        true
    }

    fn setup(&mut self) {
        let outer = HALF + WALL / 2.0;
        for (center, size) in [
            (Point::new(0.0, -outer), Shape::rect(2.0 * outer + WALL, WALL)),
            (Point::new(0.0, outer), Shape::rect(2.0 * outer + WALL, WALL)),
            (Point::new(-outer, 0.0), Shape::rect(WALL, 2.0 * outer + WALL)),
            (Point::new(outer, 0.0), Shape::rect(WALL, 2.0 * outer + WALL)),
        ] {
            self.make_sprite::<Wall>(size, center);
        }
        let blade = self.make_sprite::<MovingWall>(Shape::rect(BLADE_LENGTH, 1.0), (0, 0));
        self.blade = Some(blade);
        for i in 0..BODIES {
            let x = (i.lossy_convert() / (BODIES - 1).lossy_convert() - 0.5) * 2.0 * (HALF - 1.0);
            let body = self.make_sprite::<Body>(
                Shape::rect(0.5, 0.5),
                (x, HALF - 1.0 - (i % 3).lossy_convert() * 1.5),
            );
            self.bodies.push(body);
        }
    }

    fn update(&mut self, dt: f32) {
        self.angle -= dt * TURNS_PER_SECOND * 2.0 * PI;
        if let Some(blade) = &mut self.blade {
            let center = Point::new(self.angle.cos(), self.angle.sin()) * (BLADE_LENGTH / 2.0);
            blade.set_pose(center, self.angle);
        }
    }
}

impl LevelTest for BladeTunnel {
    fn perform_test(level: Weak<Self>) -> Result<()> {
        for _ in 0..FRAMES {
            wait_for_next_frame();
        }
        let positions: Vec<Point> =
            from_main(move || level.bodies.iter().map(|body| body.position()).collect());
        for (i, pos) in positions.into_iter().enumerate() {
            anyhow::ensure!(
                pos.x.abs() < HALF && pos.y.abs() < HALF,
                "body {i} left the box, it is at {pos:?}"
            );
        }
        Ok(())
    }
}
