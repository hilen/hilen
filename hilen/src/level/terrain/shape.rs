use std::f64::consts::TAU;

use geo::{Area, BooleanOps, Coord, LineString, MultiPolygon, Orient, Polygon, Rect, orient::Direction};

use crate::gm::{LossyConvert, flat::Point};

/// The side of the square chunks the ground is split into, in level units.
/// A carve rebuilds only the chunks its shape touches.
pub(super) const CHUNK_SIZE: f64 = 8.0;

/// The longest side of the polygon a carved circle becomes.
const CIRCLE_SIDE: f64 = 0.15;

/// Points of one ring closer than this merge after a carve, so digging the
/// same spot many times does not pile up points.
const MERGE_DISTANCE: f64 = 0.04;

/// A piece of ground smaller than this is dropped after a carve.
const MIN_AREA: f64 = 0.0005;

/// How far off a chunk border a point may be and still count as on it.
/// The boolean operations snap to their own grid, so a cut along a border
/// lands a hair away from it.
pub(super) const BORDER_TOLERANCE: f64 = 0.0001;

pub(super) type ChunkKey = (i32, i32);

pub(super) fn to_coord(point: Point) -> Coord<f64> {
    Coord {
        x: f64::from(point.x),
        y: f64::from(point.y),
    }
}

pub(super) fn to_point(coord: Coord<f64>) -> Point {
    Point::new(coord.x.lossy_convert(), coord.y.lossy_convert())
}

pub(super) fn ring(points: &[Point]) -> LineString<f64> {
    let mut ring: LineString<f64> = points.iter().copied().map(to_coord).collect();
    ring.close();
    ring
}

pub(super) fn polygon(exterior: &[Point], holes: &[Vec<Point>]) -> Polygon<f64> {
    Polygon::new(ring(exterior), holes.iter().map(|hole| ring(hole)).collect())
}

/// A circle as a polygon with sides no longer than `CIRCLE_SIDE`.
pub(super) fn circle(center: Point, radius: f32) -> Vec<Point> {
    let radius = f64::from(radius);
    let sides: f64 = (TAU * radius / CIRCLE_SIDE).ceil().clamp(12.0, 256.0);
    let count: u32 = sides.lossy_convert();
    let center = to_coord(center);
    (0..count)
        .map(|index| {
            let angle = TAU * f64::from(index) / sides;
            to_point(Coord {
                x: center.x + radius * angle.cos(),
                y: center.y + radius * angle.sin(),
            })
        })
        .collect()
}

pub(super) fn chunk_rect(key: ChunkKey) -> Rect<f64> {
    let x = f64::from(key.0) * CHUNK_SIZE;
    let y = f64::from(key.1) * CHUNK_SIZE;
    Rect::new(
        Coord { x, y },
        Coord {
            x: x + CHUNK_SIZE,
            y: y + CHUNK_SIZE,
        },
    )
}

fn chunk_coord(value: f64) -> i32 {
    (value / CHUNK_SIZE).floor().lossy_convert()
}

/// Every chunk key the rect from `min` to `max` touches.
pub(super) fn chunks_in(min: Coord<f64>, max: Coord<f64>) -> impl Iterator<Item = ChunkKey> {
    let (x0, y0) = (chunk_coord(min.x), chunk_coord(min.y));
    let (x1, y1) = (chunk_coord(max.x), chunk_coord(max.y));
    (x0..=x1).flat_map(move |x| (y0..=y1).map(move |y| (x, y)))
}

fn on_border(coord: Coord<f64>, rect: &Rect<f64>) -> bool {
    let near = |a: f64, b: f64| (a - b).abs() < BORDER_TOLERANCE;
    near(coord.x, rect.min().x)
        || near(coord.x, rect.max().x)
        || near(coord.y, rect.min().y)
        || near(coord.y, rect.max().y)
}

/// Drops every point closer than `MERGE_DISTANCE` to the point kept before
/// it. A point on the chunk border is always kept, the chunk next door has
/// the same point and the two must keep meeting there.
fn merge_ring(ring: &LineString<f64>, rect: &Rect<f64>) -> Option<LineString<f64>> {
    let coords = ring.0.split_last().map_or(&ring.0[..], |(_, open)| open);
    let mut kept: Vec<Coord<f64>> = Vec::with_capacity(coords.len());
    for &coord in coords {
        let close = kept.last().is_some_and(|last| {
            let d = *last - coord;
            d.x.hypot(d.y) < MERGE_DISTANCE
        });
        if !close || on_border(coord, rect) {
            kept.push(coord);
        }
    }
    while kept.len() > 1 {
        let first = kept[0];
        let last = kept[kept.len() - 1];
        let d = first - last;
        if d.x.hypot(d.y) >= MERGE_DISTANCE || on_border(last, rect) {
            break;
        }
        kept.pop();
    }
    (kept.len() >= 3).then(|| {
        let mut ring = LineString::new(kept);
        ring.close();
        ring
    })
}

/// Merges close points, drops crumbs and orients every polygon the same
/// way: the outside ring counter clockwise and the holes clockwise, so the
/// ground is always on the left of an edge.
pub(super) fn tidy(shape: &MultiPolygon<f64>, key: ChunkKey) -> MultiPolygon<f64> {
    let rect = chunk_rect(key);
    let polygons = shape
        .iter()
        .filter_map(|polygon| {
            let exterior = merge_ring(polygon.exterior(), &rect)?;
            let holes = polygon.interiors().iter().filter_map(|hole| merge_ring(hole, &rect)).collect();
            let polygon = Polygon::new(exterior, holes).orient(Direction::Default);
            (polygon.unsigned_area() >= MIN_AREA).then_some(polygon)
        })
        .collect();
    MultiPolygon::new(polygons)
}

pub(super) fn union_all<'a>(shapes: impl IntoIterator<Item = &'a MultiPolygon<f64>>) -> MultiPolygon<f64> {
    shapes
        .into_iter()
        .fold(MultiPolygon::new(vec![]), |sum, shape| sum.union(shape))
}

#[cfg(test)]
pub(super) fn point_count(shape: &MultiPolygon<f64>) -> usize {
    shape
        .iter()
        .map(|polygon| {
            polygon.exterior().0.len() + polygon.interiors().iter().map(|ring| ring.0.len()).sum::<usize>()
        })
        .sum()
}
