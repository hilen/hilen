use geo::{Coord, MultiPolygon, Rect};

use super::shape::{BORDER_TOLERANCE, ChunkKey, chunk_rect, to_point};
use crate::gm::{LossyConvert, flat::Point};

/// One side of the ground outline. The ground is on the left going from
/// `a` to `b`, and `normal` points out of it.
#[derive(Debug, Copy, Clone, PartialEq)]
pub(super) struct Edge {
    pub(super) a:      Point,
    pub(super) b:      Point,
    pub(super) normal: Point,
}

impl Edge {
    fn new(a: Coord<f64>, b: Coord<f64>) -> Option<Self> {
        let d = b - a;
        let length = d.x.hypot(d.y);
        (length > 1e-9).then(|| Self {
            a:      to_point(a),
            b:      to_point(b),
            normal: Point::new((d.y / length).lossy_convert(), (-d.x / length).lossy_convert()),
        })
    }

    pub(super) fn min_x(&self) -> f32 {
        self.a.x.min(self.b.x)
    }

    pub(super) fn max_x(&self) -> f32 {
        self.a.x.max(self.b.x)
    }

    /// The height of the edge at `x`, none when `x` is outside it or the
    /// edge is vertical.
    pub(super) fn y_at(&self, x: f32) -> Option<f32> {
        if (self.b.x - self.a.x).abs() < 1e-9 || x < self.min_x() || x > self.max_x() {
            return None;
        }
        let t = (x - self.a.x) / (self.b.x - self.a.x);
        Some(self.a.y + (self.b.y - self.a.y) * t)
    }

    /// The part of the edge strictly between `min` and `max` along x.
    pub(super) fn clip_x(&self, min: f32, max: f32) -> Option<(Point, Point)> {
        clip(self.a, self.b, min, max, |p| p.x)
    }

    /// The part of the edge strictly between `min` and `max` along y.
    pub(super) fn clip_y(&self, min: f32, max: f32) -> Option<(Point, Point)> {
        clip(self.a, self.b, min, max, |p| p.y)
    }
}

fn clip(a: Point, b: Point, min: f32, max: f32, axis: impl Fn(Point) -> f32) -> Option<(Point, Point)> {
    let (va, vb) = (axis(a), axis(b));
    if (vb - va).abs() < 1e-9 {
        return (va > min && va < max).then_some((a, b));
    }
    let t0 = ((min - va) / (vb - va)).clamp(0.0, 1.0);
    let t1 = ((max - va) / (vb - va)).clamp(0.0, 1.0);
    let (t0, t1) = (t0.min(t1), t0.max(t1));
    (t1 > t0).then(|| (a + (b - a) * t0, a + (b - a) * t1))
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum Side {
    Left,
    Right,
    Bottom,
    Top,
}

impl Side {
    fn line(self, rect: &Rect<f64>) -> f64 {
        match self {
            Side::Left => rect.min().x,
            Side::Right => rect.max().x,
            Side::Bottom => rect.min().y,
            Side::Top => rect.max().y,
        }
    }

    fn across(self, coord: Coord<f64>) -> f64 {
        match self {
            Side::Left | Side::Right => coord.x,
            Side::Bottom | Side::Top => coord.y,
        }
    }

    fn along(self, coord: Coord<f64>) -> f64 {
        match self {
            Side::Left | Side::Right => coord.y,
            Side::Bottom | Side::Top => coord.x,
        }
    }

    fn neighbor(self, key: ChunkKey) -> ChunkKey {
        match self {
            Side::Left => (key.0 - 1, key.1),
            Side::Right => (key.0 + 1, key.1),
            Side::Bottom => (key.0, key.1 - 1),
            Side::Top => (key.0, key.1 + 1),
        }
    }

    fn opposite(self) -> Self {
        match self {
            Side::Left => Side::Right,
            Side::Right => Side::Left,
            Side::Bottom => Side::Top,
            Side::Top => Side::Bottom,
        }
    }

    fn of(a: Coord<f64>, b: Coord<f64>, rect: &Rect<f64>) -> Option<Self> {
        [Side::Left, Side::Right, Side::Bottom, Side::Top].into_iter().find(|side| {
            let line = side.line(rect);
            (side.across(a) - line).abs() < BORDER_TOLERANCE
                && (side.across(b) - line).abs() < BORDER_TOLERANCE
        })
    }
}

fn rings(shape: &MultiPolygon<f64>) -> impl Iterator<Item = (Coord<f64>, Coord<f64>)> {
    shape.iter().flat_map(|polygon| {
        std::iter::once(polygon.exterior())
            .chain(polygon.interiors())
            .flat_map(|ring| ring.lines().map(|line| (line.start, line.end)))
    })
}

/// The spans along `side` of the chunk at `key` its ground covers.
fn border_spans(shape: &MultiPolygon<f64>, key: ChunkKey, side: Side) -> Vec<(f64, f64)> {
    let rect = chunk_rect(key);
    rings(shape)
        .filter(|&(a, b)| Side::of(a, b, &rect) == Some(side))
        .map(|(a, b)| {
            let (a, b) = (side.along(a), side.along(b));
            (a.min(b) - BORDER_TOLERANCE, a.max(b) + BORDER_TOLERANCE)
        })
        .collect()
}

/// What is left of the span from `from` to `to` once every covered span
/// is cut out of it.
fn uncovered(from: f64, to: f64, covered: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let mut left = vec![(from, to)];
    for &(c0, c1) in covered {
        left = left
            .into_iter()
            .flat_map(|(s0, s1)| {
                let mut parts = vec![];
                if c0 > s0 {
                    parts.push((s0, c0.min(s1)));
                }
                if c1 < s1 {
                    parts.push((c1.max(s0), s1));
                }
                parts
            })
            .filter(|(s0, s1)| s1 - s0 > BORDER_TOLERANCE)
            .collect();
    }
    left
}

/// The edges of a chunk's ground a box can touch. A chunk cuts the ground
/// along its square, and where the chunk next door has ground on the other
/// side of that cut, the cut is not a real side of the ground and is left
/// out.
pub(super) fn chunk_edges<'a>(
    key: ChunkKey,
    solid: &MultiPolygon<f64>,
    neighbor: impl Fn(ChunkKey) -> Option<&'a MultiPolygon<f64>>,
) -> Vec<Edge> {
    let rect = chunk_rect(key);
    let mut edges = vec![];
    for (a, b) in rings(solid) {
        let Some(side) = Side::of(a, b, &rect) else {
            edges.extend(Edge::new(a, b));
            continue;
        };
        let covered = neighbor(side.neighbor(key))
            .map_or_default(|shape| border_spans(shape, side.neighbor(key), side.opposite()));
        let (from, to) = (side.along(a), side.along(b));
        for (s0, s1) in uncovered(from.min(to), from.max(to), &covered) {
            let at = |value: f64| a + (b - a) * ((value - from) / (to - from));
            if from < to {
                edges.extend(Edge::new(at(s0), at(s1)));
            } else {
                edges.extend(Edge::new(at(s1), at(s0)));
            }
        }
    }
    edges
}
