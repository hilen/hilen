use std::f32::consts::PI;

use hilen::{
    gm::{LossyConvert, Shape},
    level::{Body, LevelCreation, LevelManager, LevelSetup, MovingWall, SpriteTemplates, Wall, level},
    refs::Weak,
    time::Instant,
    ui::{Color, Point},
};

use crate::interface::palette::{ACCENT_END, ACCENT_START};

pub const RADIUS: f32 = 14.0;
const WALL_THICKNESS: f32 = 1.2;
const SEGMENTS: usize = 40;
// Half width of the opening at the top, as an angle from straight up.
const GAP: f32 = PI / 7.0;
const BLADES: usize = 3;
const BLADE_LENGTH: f32 = RADIUS * 0.78;
const BLADE_THICKNESS: f32 = 1.0;
const HUB_RADIUS: f32 = 1.6;
const TURN_SECONDS: f32 = 3.0;
const DROP_SECONDS: f32 = 0.25;
const MAX_OBJECTS: usize = 100;
// The pipe walls rise straight up from the gap edges. Far taller than
// any screen, so the pipe always runs off the top.
const PIPE_LENGTH: f32 = 400.0;

/// A round chamber with an opening on top and three blades turning in
/// the middle. Random objects drop through the opening onto the blades
/// and get pushed around. A tap inside the chamber drops one there.
/// Dropping stops once the cap is reached, nothing is ever removed.
#[level]
#[derive(Default)]
pub struct ChamberLevel {
    /// Where new objects appear, above the window top. The landing sets
    /// it from the camera, the level does not know the screen.
    pub spawn_y: f32,

    blades:    Vec<Weak<MovingWall>>,
    objects:   Vec<Weak<Body>>,
    started:   Option<Instant>,
    last_drop: Option<Instant>,
}

impl ChamberLevel {
    /// The ring and the pipe are one thick U shaped stroke with a gap at
    /// the top, cut into slices that share their edge points exactly,
    /// so nothing overlaps and the corner into the pipe is a true miter.
    /// Each slice is its own wall with its own color step, that is how
    /// a one color per shape drawer shows a gradient.
    fn add_walls(&mut self) {
        let outer = RADIUS + WALL_THICKNESS / 2.0;
        let inner = RADIUS - WALL_THICKNESS / 2.0;
        // Pipe faces, measured from the center line. The pipe is as wide
        // as the gap in the ring center line.
        let face_out = RADIUS * GAP.sin() + WALL_THICKNESS / 2.0;
        let face_in = RADIUS * GAP.sin() - WALL_THICKNESS / 2.0;
        // Where each arc meets its pipe face, as an angle from straight up.
        let end_out = (face_out / outer).asin();
        let end_in = (face_in / inner).asin();

        // Arc slices, from the right pipe corner clockwise around the
        // bottom to the left pipe corner. Angles are from straight up on
        // the right side, mirrored for the left.
        let arc = |from_up: f32| -> (Point, Point) {
            let a = PI / 2.0 - from_up;
            (
                Point::new(outer * a.cos(), outer * a.sin()),
                Point::new(inner * a.cos(), inner * a.sin()),
            )
        };
        let span_out = 2.0 * PI - 2.0 * end_out;
        let span_in = 2.0 * PI - 2.0 * end_in;
        for i in 0..SEGMENTS {
            let t0 = i.lossy_convert() / SEGMENTS.lossy_convert();
            let t1 = (i + 1).lossy_convert() / SEGMENTS.lossy_convert();
            let (o0, _) = arc(end_out + span_out * t0);
            let (o1, _) = arc(end_out + span_out * t1);
            let (_, i0) = arc(end_in + span_in * t0);
            let (_, i1) = arc(end_in + span_in * t1);
            let mut slice = self.make_sprite::<Wall>(Shape::Polyline(vec![o0, o1, i1, i0]), (0, 0));
            slice.set_color(ring_color(t0));
        }

        // The pipes. The corner points are the arc ends, so the pipe
        // meets the ring on the same outline.
        let (o_right, _) = arc(end_out);
        let (_, i_right) = arc(end_in);
        let top = PIPE_LENGTH;
        for side in [-1.0_f32, 1.0] {
            let o = Point::new(side * o_right.x, o_right.y);
            let i = Point::new(side * i_right.x, i_right.y);
            let mut pipe = self.make_sprite::<Wall>(
                Shape::Polyline(vec![o, Point::new(o.x, top), Point::new(i.x, top), i]),
                (0, 0),
            );
            pipe.set_color(ring_color(if side > 0.0 { 0.0 } else { 1.0 }));
        }
    }

    fn add_blades(&mut self) {
        let mut hub = self.make_sprite::<Wall>(Shape::Circle(HUB_RADIUS), (0, 0));
        hub.set_color(ACCENT_END);
        for _ in 0..BLADES {
            let mut blade =
                self.make_sprite::<MovingWall>(Shape::rect(BLADE_LENGTH, BLADE_THICKNESS), (0, 0));
            blade.set_color(ACCENT_START);
            self.blades.push(blade);
        }
        self.turn_blades(0.0);
    }

    fn turn_blades(&mut self, angle: f32) {
        let step = 2.0 * PI / BLADES.lossy_convert();
        for (i, blade) in self.blades.iter_mut().enumerate() {
            let a = angle + step * i.lossy_convert();
            let center = Point::new(a.cos(), a.sin()) * (BLADE_LENGTH / 2.0);
            blade.set_pose(center, a);
        }
    }

    fn drop_object(&mut self, pos: Point) {
        if self.objects.len() >= MAX_OBJECTS {
            return;
        }
        let size = fastrand::f32() * 0.7 + 0.5;
        let shape = match fastrand::u8(0..4) {
            0 => Shape::Circle(size / 2.0),
            1 => Shape::triangle(
                (-size / 2.0, -size / 2.0),
                (size / 2.0, -size / 2.0),
                (0.0, size / 2.0),
            ),
            _ => Shape::rect(size, size),
        };
        let crate_box = matches!(shape, Shape::Rect(_)) && fastrand::bool();
        let mut body = self.make_sprite::<Body>(shape, pos);
        if crate_box {
            body.set_image("crate_box.png");
        } else {
            body.set_color(Color::random());
        }
        self.objects.push(body);
    }

    fn drop_from_top(&mut self) {
        let inner = RADIUS * GAP.sin() - WALL_THICKNESS - 1.0;
        let x = (fastrand::f32() - 0.5) * 2.0 * inner;
        let y = self.spawn_y.max(RADIUS + 4.0);
        self.drop_object(Point::new(x, y));
    }
}

impl LevelSetup for ChamberLevel {
    fn needs_physics(&self) -> bool {
        true
    }

    fn setup(&mut self) {
        self.add_walls();
        self.add_blades();
        let now = Instant::now();
        self.started = Some(now);
        self.last_drop = Some(now);

        // A tap inside the chamber drops an object there, outside does nothing.
        self.on_tap.val(|pos| {
            if pos.x.hypot(pos.y) < RADIUS - 2.0 {
                LevelManager::downcast_level::<Self>().drop_object(pos);
            }
        });
    }

    fn update(&mut self) {
        let now = Instant::now();
        let Some(started) = self.started else {
            return;
        };
        let turns = started.elapsed().as_secs_f32() / TURN_SECONDS;
        self.turn_blades(-turns * 2.0 * PI);

        if self
            .last_drop
            .is_none_or(|last| now.duration_since(last).as_secs_f32() >= DROP_SECONDS)
        {
            self.last_drop = Some(now);
            self.drop_from_top();
        }
    }
}

fn ring_color(t: f32) -> Color {
    let lerp = |a: f32, b: f32| a + (b - a) * t;
    Color::rgb(
        lerp(ACCENT_START.r, ACCENT_END.r),
        lerp(ACCENT_START.g, ACCENT_END.g),
        lerp(ACCENT_START.b, ACCENT_END.b),
    )
}
