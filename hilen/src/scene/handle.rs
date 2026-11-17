use std::f32::consts::{PI, TAU};

use crate::{
    deps::{refs::Weak, vents::Event},
    gm::{
        LossyConvert,
        color::Color,
        flat::{Point, Size},
        volume::{Quat, Ray, Vec3},
    },
    scene::{AXIS_COLORS, Camera, Line3, Node},
};

/// The part a drag holds, drawn in this color in place of its axis color.
const ACTIVE_COLOR: Color = Color::hex("#ffd23f");
/// A press this many pixels or closer to a part takes it.
const REACH: f32 = 8.0;
/// Where the parts sit along an axis, in arrow lengths.
const SHAFT_START: f32 = 0.25;
const SHAFT_END: f32 = 1.0;
const TIP: f32 = 1.22;
const HEAD_RADIUS: f32 = 0.07;
const BOX_AT: f32 = 0.6;
const BOX_HALF: f32 = 0.06;
const RING_RADIUS: f32 = 0.8;
const RING_SEGMENTS: usize = 48;
const HEAD_SEGMENTS: usize = 8;
/// A scale drag never goes below this factor, a part cannot turn inside out.
const MIN_FACTOR: f32 = 0.01;

/// What a part of a handle does along its axis.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HandleMode {
    /// An arrow, moves along the axis.
    Move,
    /// A ring, turns about the axis.
    Turn,
    /// A box on the arrow, scales along the axis.
    Scale,
}

/// One of the 9 parts of a handle, a mode on an axis, 0 for x, 1 for y
/// and 2 for z.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct HandlePart {
    pub mode: HandleMode,
    pub axis: usize,
}

/// Steps a drag snaps to, none for a smooth drag.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct HandleSnap {
    /// World units along an axis.
    pub moves:  Option<f32>,
    /// Radians about an axis.
    pub turns:  Option<f32>,
    /// Steps of the scale factor, 0.1 makes 1.1, 1.2 and so on.
    pub scales: Option<f32>,
}

/// A drag on a handle, counted from where it began, after the snap.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HandleDrag {
    pub part:   HandlePart,
    /// World units moved along the axis, radians turned about it, right
    /// handed, or the factor of the scale along it, 1 at the start.
    pub amount: f32,
    /// The direction of the axis in the world.
    pub axis:   Vec3,
    /// The last change, the press let go.
    pub ended:  bool,
}

/// What a drag keeps from its start.
#[derive(Clone, Copy, Debug)]
struct Grab {
    part:   HandlePart,
    center: Vec3,
    axis:   Vec3,
    /// Where along the axis the press met it, for a move and a scale.
    along:  f32,
    /// The angle of the press around the center on the screen and the
    /// turn so far, for a turn.
    angle:  f32,
    turned: f32,
    /// Whether the axis points at the eye, a turn on the screen goes the
    /// other way about it then.
    facing: bool,
    last:   f32,
}

/// A handle to move, turn and scale a node with the mouse or a finger:
/// 3 arrows to move along x, y and z, 3 rings to turn about them and 3
/// boxes on the arrows to scale along them. Put one in
/// `SceneBase::handle`. It draws over everything at the same size on the
/// screen at any distance, and a `SceneView` hands it a drag that begins
/// on 1 of its parts in place of turning the camera.
///
/// The handle changes nothing by itself. It tells `on_drag` the part,
/// the amount from the start of the drag and the axis, on every move and
/// once more at the end, and the app moves, turns or scales what it
/// wants, a node or a line of code. A node has 1 scale for all 3 axes, so
/// a scale along 1 axis is for the app to apply.
pub struct TransformHandle {
    /// Where the handle stands, while it follows no node.
    pub position: Vec3,
    /// How its axes are turned, while it follows no node or `local` is
    /// off. The world axes by default.
    pub rotation: Quat,
    /// The node the handle stands on, it follows its place every frame.
    pub node:     Weak<dyn Node>,
    /// The axes turn with the node.
    pub local:    bool,
    /// The length of an arrow on the screen, in pixels.
    pub size:     f32,
    /// The modes shown, all 3 by default.
    pub modes:    Vec<HandleMode>,
    pub snap:     HandleSnap,
    pub on_drag:  Event<HandleDrag>,

    grab: Option<Grab>,
}

impl Default for TransformHandle {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            node:     Weak::default(),
            local:    false,
            size:     110.0,
            modes:    vec![HandleMode::Move, HandleMode::Turn, HandleMode::Scale],
            snap:     HandleSnap::default(),
            on_drag:  Event::default(),
            grab:     None,
        }
    }
}

impl TransformHandle {
    /// A handle standing on `node`, with the world axes.
    pub fn on(node: Weak<dyn Node>) -> Self {
        Self {
            node,
            ..Self::default()
        }
    }

    /// The part a drag holds now.
    pub fn active(&self) -> Option<HandlePart> {
        self.grab.map(|grab| grab.part)
    }

    /// Where the handle stands now.
    pub fn center(&self) -> Vec3 {
        if self.node.is_ok() {
            self.node.position()
        } else {
            self.position
        }
    }

    /// The 3 axes, unit long.
    pub fn axes(&self) -> [Vec3; 3] {
        let rotation = if self.local && self.node.is_ok() {
            self.node.rotation()
        } else {
            self.rotation
        };
        [Vec3::X, Vec3::Y, Vec3::Z].map(|axis| (rotation * axis).normalize())
    }

    /// The world length of an arrow, `size` pixels at the center.
    fn length(&self, camera: &Camera, area: Size) -> f32 {
        camera.units_per_pixel(self.center(), area) * self.size
    }

    /// Every part the handle shows with its pieces in the world.
    fn parts(&self, camera: &Camera, area: Size) -> Vec<(HandlePart, Vec<[Vec3; 2]>)> {
        let center = self.center();
        let length = self.length(camera, area);
        let axes = self.axes();
        let mut parts = vec![];
        for mode in [HandleMode::Turn, HandleMode::Move, HandleMode::Scale] {
            if !self.modes.contains(&mode) {
                continue;
            }
            for (axis, direction) in axes.iter().enumerate() {
                let [side, other] = [axes[(axis + 1) % 3], axes[(axis + 2) % 3]];
                let pieces = match mode {
                    HandleMode::Move => arrow(center, *direction, side, other, length),
                    HandleMode::Turn => ring(center, side, other, length * RING_RADIUS),
                    HandleMode::Scale => cube(center + *direction * length * BOX_AT, axes, length * BOX_HALF),
                };
                parts.push((HandlePart { mode, axis }, pieces));
            }
        }
        parts
    }

    /// The lines the handle draws with, on top of everything.
    pub(crate) fn lines(&self, camera: &Camera, area: Size) -> Vec<Line3> {
        let active = self.active();
        self.parts(camera, area)
            .into_iter()
            .flat_map(|(part, pieces)| {
                let color = if active == Some(part) {
                    ACTIVE_COLOR
                } else {
                    AXIS_COLORS[part.axis]
                };
                let width = match part.mode {
                    HandleMode::Move => 3.0,
                    HandleMode::Turn | HandleMode::Scale => 2.5,
                };
                pieces
                    .into_iter()
                    .map(move |[from, to]| Line3::new(from, to, color).width(width).on_top())
            })
            .collect()
    }

    /// The part under `pixel` of a view drawn over `area`, a box before an
    /// arrow before a ring, since a box sits on an arrow and a ring
    /// crosses 2 arrows.
    pub fn part_at(&self, pixel: Point, camera: &Camera, area: Size) -> Option<HandlePart> {
        self.parts(camera, area)
            .into_iter()
            .filter_map(|(part, pieces)| {
                let distance = pieces
                    .iter()
                    .filter_map(|[from, to]| {
                        let from = camera.screen_point(*from, area)?;
                        let to = camera.screen_point(*to, area)?;
                        Some(segment_distance(pixel, from, to))
                    })
                    .reduce(f32::min)?;
                let rank: u8 = match part.mode {
                    HandleMode::Scale => 0,
                    HandleMode::Move => 1,
                    HandleMode::Turn => 2,
                };
                (distance <= REACH).then_some((rank, distance, part))
            })
            .min_by(|a, b| a.0.cmp(&b.0).then(a.1.total_cmp(&b.1)))
            .map(|(_, _, part)| part)
    }

    /// Starts a drag when `pixel` is on a part, and says whether it was.
    pub fn press(&mut self, pixel: Point, camera: &Camera, area: Size) -> bool {
        let Some(part) = self.part_at(pixel, camera, area) else {
            return false;
        };
        let center = self.center();
        let axis = self.axes()[part.axis];
        let ray = camera.ray(pixel, area);
        let screen_center = camera.screen_point(center, area).unwrap_or(pixel);
        self.grab = Some(Grab {
            part,
            center,
            axis,
            along: along_axis(ray, center, axis).unwrap_or(0.0),
            angle: screen_angle(pixel, screen_center),
            turned: 0.0,
            facing: axis.dot(-ray.direction) > 0.0,
            last: 0.0,
        });
        true
    }

    /// The press moved to `pixel`, tells `on_drag` the amount.
    pub fn drag_to(&mut self, pixel: Point, camera: &Camera, area: Size) {
        if let Some(drag) = self.follow(pixel, camera, area, false) {
            self.on_drag.trigger(drag);
        }
    }

    /// The press let go at `pixel`, tells `on_drag` the last amount with
    /// `ended` set.
    pub fn release(&mut self, pixel: Point, camera: &Camera, area: Size) {
        let drag = self.follow(pixel, camera, area, true);
        self.grab = None;
        if let Some(drag) = drag {
            self.on_drag.trigger(drag);
        }
    }

    fn follow(&mut self, pixel: Point, camera: &Camera, area: Size, ended: bool) -> Option<HandleDrag> {
        let snap = self.snap;
        let grab = self.grab.as_mut()?;
        let ray = camera.ray(pixel, area);
        let raw = match grab.part.mode {
            // A ray along the axis meets it nowhere, the amount stays.
            HandleMode::Move => {
                along_axis(ray, grab.center, grab.axis).map_or(grab.last, |at| at - grab.along)
            }
            HandleMode::Scale => along_axis(ray, grab.center, grab.axis)
                .filter(|_| grab.along.abs() > f32::EPSILON)
                .map_or(grab.last, |at| (at / grab.along).max(MIN_FACTOR)),
            HandleMode::Turn => {
                let screen_center = camera.screen_point(grab.center, area).unwrap_or(pixel);
                let angle = screen_angle(pixel, screen_center);
                grab.turned += wrapped(angle - grab.angle);
                grab.angle = angle;
                if grab.facing { grab.turned } else { -grab.turned }
            }
        };
        grab.last = raw;
        let step = match grab.part.mode {
            HandleMode::Move => snap.moves,
            HandleMode::Turn => snap.turns,
            HandleMode::Scale => snap.scales,
        };
        let amount = step.filter(|step| *step > 0.0).map_or(raw, |step| (raw / step).round() * step);
        Some(HandleDrag {
            part: grab.part,
            amount,
            axis: grab.axis,
            ended,
        })
    }
}

/// An arrow along `direction`: the shaft and a cone of lines at its tip.
fn arrow(center: Vec3, direction: Vec3, side: Vec3, other: Vec3, length: f32) -> Vec<[Vec3; 2]> {
    let base = center + direction * length * SHAFT_END;
    let tip = center + direction * length * TIP;
    let mut pieces = vec![[center + direction * length * SHAFT_START, tip]];
    let rim = circle(base, side, other, length * HEAD_RADIUS, HEAD_SEGMENTS);
    for [from, to] in &rim {
        pieces.push([*from, tip]);
        pieces.push([*from, *to]);
    }
    pieces
}

/// A ring around `center` in the plane of `side` and `other`.
fn ring(center: Vec3, side: Vec3, other: Vec3, radius: f32) -> Vec<[Vec3; 2]> {
    circle(center, side, other, radius, RING_SEGMENTS)
}

fn circle(center: Vec3, side: Vec3, other: Vec3, radius: f32, segments: usize) -> Vec<[Vec3; 2]> {
    let point = |index: usize| {
        let index: f32 = index.lossy_convert();
        let count: f32 = segments.lossy_convert();
        let angle = TAU * index / count;
        center + (side * angle.cos() + other * angle.sin()) * radius
    };
    (0..segments).map(|index| [point(index), point(index + 1)]).collect()
}

/// The 12 edges of a cube of `half` around `center` turned by `axes`.
fn cube(center: Vec3, axes: [Vec3; 3], half: f32) -> Vec<[Vec3; 2]> {
    let corner = |x: f32, y: f32, z: f32| center + (axes[0] * x + axes[1] * y + axes[2] * z) * half;
    let mut pieces = vec![];
    for a in [-1.0, 1.0] {
        for b in [-1.0, 1.0] {
            pieces.push([corner(-1.0, a, b), corner(1.0, a, b)]);
            pieces.push([corner(a, -1.0, b), corner(a, 1.0, b)]);
            pieces.push([corner(a, b, -1.0), corner(a, b, 1.0)]);
        }
    }
    pieces
}

/// How far along the axis through `center` the point nearest to `ray`
/// is, none for a ray along the axis.
fn along_axis(ray: Ray, center: Vec3, axis: Vec3) -> Option<f32> {
    let direction = ray.direction.normalize();
    let between = ray.origin - center;
    let cos = direction.dot(axis);
    let sin_squared = 1.0 - cos * cos;
    if sin_squared < 1e-6 {
        return None;
    }
    Some((axis.dot(between) - cos * direction.dot(between)) / sin_squared)
}

/// The angle of `pixel` around `center` on the screen, counter clockwise
/// with y up.
fn screen_angle(pixel: Point, center: Point) -> f32 {
    (center.y - pixel.y).atan2(pixel.x - center.x)
}

/// An angle step brought into a half turn either way, so crossing the
/// left of the center does not jump a whole turn.
fn wrapped(angle: f32) -> f32 {
    (angle + PI).rem_euclid(TAU) - PI
}

fn segment_distance(point: Point, from: Point, to: Point) -> f32 {
    let line = to - from;
    let length = line.x * line.x + line.y * line.y;
    let t = if length > 0.0 {
        (((point.x - from.x) * line.x + (point.y - from.y) * line.y) / length).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let nearest = Point::new(from.x + line.x * t, from.y + line.y * t);
    let gap = point - nearest;
    (gap.x * gap.x + gap.y * gap.y).sqrt()
}

#[cfg(test)]
mod test {
    use super::*;

    const AREA: Size = Size::new(600.0, 600.0);

    /// Looking at the origin from 10 units in front, so a handle of 100
    /// pixels there has arrows 1.92 units long, 0.019 units a pixel.
    fn front() -> Camera {
        Camera {
            position: Vec3::new(0.0, 0.0, 10.0),
            target: Vec3::ZERO,
            ..Camera::default()
        }
    }

    fn new_handle() -> TransformHandle {
        TransformHandle {
            size: 100.0,
            ..TransformHandle::default()
        }
    }

    fn unit() -> f32 {
        front().units_per_pixel(Vec3::ZERO, AREA)
    }

    fn part(mode: HandleMode, axis: usize) -> HandlePart {
        HandlePart { mode, axis }
    }

    // The center is at 300, 300 and x goes right, y goes up on the
    // screen, z comes out of it.
    #[test]
    fn a_press_finds_the_part_under_it() {
        let handle = new_handle();
        let camera = front();
        assert_eq!(
            handle.part_at(Point::new(340.0, 300.0), &camera, AREA),
            Some(part(HandleMode::Move, 0))
        );
        assert_eq!(
            handle.part_at(Point::new(300.0, 225.0), &camera, AREA),
            Some(part(HandleMode::Move, 1))
        );
        // The box sits on the arrow at 0.6 of it and wins there.
        assert_eq!(
            handle.part_at(Point::new(360.0, 300.0), &camera, AREA),
            Some(part(HandleMode::Scale, 0))
        );
        // The ring about z lies flat on the screen, 80 pixels round.
        let diagonal = 80.0 / 2.0_f32.sqrt();
        assert_eq!(
            handle.part_at(Point::new(300.0 + diagonal, 300.0 - diagonal), &camera, AREA),
            Some(part(HandleMode::Turn, 2))
        );
        assert_eq!(handle.part_at(Point::new(330.0, 330.0), &camera, AREA), None);
    }

    #[test]
    fn a_move_follows_the_pointer_along_the_axis_and_snaps() {
        let mut handle = new_handle();
        let camera = front();
        assert!(handle.press(Point::new(340.0, 300.0), &camera, AREA));
        let drag = handle.follow(Point::new(390.0, 300.0), &camera, AREA, false).expect("held");
        assert_eq!(
            drag.part,
            HandlePart {
                mode: HandleMode::Move,
                axis: 0,
            }
        );
        assert!(
            (drag.amount - 50.0 * unit()).abs() < 1e-3,
            "moved {}",
            drag.amount
        );
        assert_eq!(drag.axis, Vec3::X);

        handle.snap.moves = Some(0.5);
        let snapped = handle.follow(Point::new(390.0, 300.0), &camera, AREA, true).expect("held");
        assert!((snapped.amount - 1.0).abs() < 1e-6 && snapped.ended);
    }

    // A turn about z, which points at the eye, a quarter turn counter
    // clockwise on the screen is a quarter turn about z by the right hand.
    // Seen from behind the same drag turns the other way.
    #[test]
    fn a_turn_goes_by_the_right_hand_and_crosses_the_left_side() {
        let mut handle = new_handle();
        let camera = front();
        let at = |angle: f32| Point::new(300.0 + 80.0 * angle.cos(), 300.0 - 80.0 * angle.sin());
        assert!(handle.press(at(0.8), &camera, AREA));
        assert_eq!(handle.active(), Some(part(HandleMode::Turn, 2)));
        handle.follow(at(0.8 + 1.2), &camera, AREA, false);
        handle.follow(at(0.8 + 2.4), &camera, AREA, false);
        let drag = handle.follow(at(0.8 + 3.0), &camera, AREA, false).expect("held");
        assert!((drag.amount - 3.0).abs() < 1e-3, "turned {}", drag.amount);

        let behind = Camera {
            position: Vec3::new(0.0, 0.0, -10.0),
            ..front()
        };
        let mut from_behind = new_handle();
        let mirrored = |angle: f32| Point::new(300.0 - 80.0 * angle.cos(), 300.0 - 80.0 * angle.sin());
        assert!(from_behind.press(mirrored(0.8), &behind, AREA));
        let drag = from_behind.follow(mirrored(0.8 + 1.0), &behind, AREA, false).expect("held");
        assert!((drag.amount - 1.0).abs() < 1e-3, "turned {}", drag.amount);
    }

    #[test]
    fn a_scale_is_the_ratio_of_the_distances_from_the_center() {
        let mut handle = new_handle();
        let camera = front();
        assert!(handle.press(Point::new(360.0, 300.0), &camera, AREA));
        let drag = handle.follow(Point::new(420.0, 300.0), &camera, AREA, false).expect("held");
        assert_eq!(drag.part.mode, HandleMode::Scale);
        assert!((drag.amount - 2.0).abs() < 1e-3, "scaled {}", drag.amount);
        let flat = handle.follow(Point::new(200.0, 300.0), &camera, AREA, false).expect("held");
        assert!((flat.amount - MIN_FACTOR).abs() < 1e-6);
    }

    // The arrows stay 100 pixels long wherever the camera stands.
    #[test]
    fn the_handle_keeps_its_size_on_the_screen() {
        let handle = new_handle();
        for distance in [5.0, 10.0, 40.0] {
            let camera = Camera {
                position: Vec3::new(0.0, 0.0, distance),
                ..front()
            };
            let length = handle.length(&camera, AREA);
            let end = camera.screen_point(Vec3::X * length, AREA).expect("in front");
            assert!(
                (end.x - 400.0).abs() < 0.01,
                "{distance} away the arrow ends at {end:?}"
            );
        }
    }

    #[test]
    fn a_press_off_the_handle_takes_nothing() {
        let mut handle = new_handle();
        assert!(!handle.press(Point::new(500.0, 500.0), &front(), AREA));
        assert_eq!(handle.active(), None);
        assert!(handle.follow(Point::new(510.0, 500.0), &front(), AREA, false).is_none());
    }
}
