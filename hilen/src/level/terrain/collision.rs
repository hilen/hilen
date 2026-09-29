use super::{OneWay, Terrain, edges::Edge};
use crate::{
    gm::{LossyConvert, flat::Point},
    level::BoxMove,
};

/// How far a box stays off an edge it stopped at, like the tile map's.
const EDGE: f32 = 0.001;

/// How far inside a box an edge may start and still count as ahead of it.
const TOUCH: f32 = EDGE * 0.5;

/// The shortest hop a long move is split into.
const MIN_HOP: f32 = 0.05;

/// What holds a box up.
#[derive(Debug, Copy, Clone, PartialEq)]
enum Support {
    Ground,
    OneWay(usize),
}

/// Where a falling box first meets an edge that is too steep to walk on.
#[derive(Debug, Copy, Clone)]
struct Contact {
    distance: f32,
    normal:   Point,
    /// The box sits on a corner of the ground, not on a slope that runs
    /// on under its bottom corner, so it stays instead of sliding.
    rests:    bool,
}

impl OneWay {
    fn y_at(&self, x: f32) -> Option<f32> {
        let (min, max) = (self.a.x.min(self.b.x), self.a.x.max(self.b.x));
        if (self.b.x - self.a.x).abs() < 1e-9 || x < min || x > max {
            return None;
        }
        Some(self.a.y + (self.b.y - self.a.y) * (x - self.a.x) / (self.b.x - self.a.x))
    }
}

impl Terrain {
    fn slope_radians(&self) -> f32 {
        self.slope_limit.clamp(0.0, 89.0).to_radians()
    }

    /// Whether a box walks on the edge, it faces up and is no steeper
    /// than `slope_limit`.
    pub(super) fn walkable(&self, edge: &Edge) -> bool {
        edge.normal.y > 0.0 && edge.normal.y >= self.slope_radians().cos() - 1e-6
    }

    /// The highest ground or one way segment under the point at `x`
    /// between `low` and `high`. `one_way` picks which segments count.
    fn support(
        &self,
        x: f32,
        low: f32,
        high: f32,
        one_way: impl Fn(usize, f32) -> bool,
    ) -> Option<(f32, Support)> {
        let ground = self
            .edges_near(Point::new(x - TOUCH, low), Point::new(x + TOUCH, high))
            .filter(|edge| self.walkable(edge))
            .filter_map(|edge| edge.y_at(x))
            .filter(|y| (low..=high).contains(y))
            .map(|y| (y, Support::Ground));
        let segments = self.one_way.iter().enumerate().filter_map(|(index, segment)| {
            let y = segment.y_at(x)?;
            ((low..=high).contains(&y) && one_way(index, y)).then_some((y, Support::OneWay(index)))
        });
        ground.chain(segments).max_by(|a, b| a.0.total_cmp(&b.0))
    }

    /// How far a box moves along x before its side meets an edge too
    /// steep to walk on, none when the whole move is free.
    fn sweep_x(&self, pos: Point, half: Point, dx: f32) -> Option<f32> {
        let dir = dx.signum();
        let face = pos.x + half.x * dir;
        let (bottom, top) = (pos.y - half.y + TOUCH, pos.y + half.y - TOUCH);
        let far = face + dx + EDGE * dir;
        self.edges_near(
            Point::new(face.min(far) - TOUCH, bottom),
            Point::new(face.max(far) + TOUCH, top),
        )
        .filter(|edge| !self.walkable(edge) && edge.normal.x * dir < -1e-6)
        .filter_map(|edge| {
            let (p, q) = edge.clip_y(bottom, top)?;
            let near = if dir > 0.0 { p.x.min(q.x) } else { p.x.max(q.x) };
            let gap = (near - face) * dir;
            (gap >= -TOUCH).then_some((gap - EDGE).max(0.0))
        })
        .filter(|distance| *distance < dx.abs())
        .min_by(f32::total_cmp)
    }

    /// How far a box moves up before its top meets the ground above.
    fn sweep_up(&self, pos: Point, half: Point, dist: f32) -> Option<f32> {
        let top = pos.y + half.y;
        let (left, right) = (pos.x - half.x + TOUCH, pos.x + half.x - TOUCH);
        self.edges_near(
            Point::new(left, top - TOUCH),
            Point::new(right, top + dist + EDGE),
        )
        .filter(|edge| edge.normal.y <= 1e-6)
        .filter_map(|edge| {
            let (p, q) = edge.clip_x(left, right)?;
            let gap = p.y.min(q.y) - top;
            (gap >= -TOUCH).then_some((gap - EDGE).max(0.0))
        })
        .filter(|distance| *distance < dist)
        .min_by(f32::total_cmp)
    }

    /// Where a box moving down first meets an edge too steep to walk on.
    /// Walkable edges only hold the middle of the bottom, see `support`.
    fn sweep_down(&self, pos: Point, half: Point, dist: f32) -> Option<Contact> {
        let bottom = pos.y - half.y;
        let (left, right) = (pos.x - half.x + TOUCH, pos.x + half.x - TOUCH);
        self.edges_near(
            Point::new(left, bottom - dist - EDGE),
            Point::new(right, bottom + TOUCH),
        )
        .filter(|edge| !self.walkable(edge) && edge.normal.y >= -1e-6)
        .filter_map(|edge| {
            let (p, q) = edge.clip_x(left, right)?;
            let high = if p.y >= q.y { p } else { q };
            let gap = bottom - high.y;
            let inside = high.x > left + TOUCH && high.x < right - TOUCH;
            (gap >= -TOUCH).then_some(Contact {
                distance: (gap - EDGE).max(0.0),
                normal:   edge.normal,
                rests:    inside || edge.normal.x.abs() < 1e-3,
            })
        })
        .filter(|contact| contact.distance <= dist)
        .min_by(|a, b| a.distance.total_cmp(&b.distance).then(b.rests.cmp(&a.rests)))
    }

    /// Moves a box with this center and half size by `delta`, first along
    /// x and then along y in short hops, the contract of
    /// `TileMap::move_box`. Every sweep covers the whole hop, so a fast
    /// box never skips a thin edge.
    ///
    /// The middle of the box bottom rides every edge flatter than
    /// `slope_limit`. Walking follows such ground up and down, and down
    /// only while the box stood on it and is not moving up, so it sticks
    /// to a downhill slope instead of bouncing off it. A steeper edge is a
    /// wall to the box sides and top. A box falling onto one slides down
    /// it, and one landing on a corner of the ground stays there.
    /// A one way segment stops the box only when it falls onto it from
    /// above, and never with `drop_through`.
    pub fn move_box(&self, center: Point, half: Point, delta: Point, drop_through: bool) -> BoxMove {
        let mut result = BoxMove {
            position: center,
            ..BoxMove::default()
        };

        let hop_length = half.x.min(half.y).max(MIN_HOP);
        let longest = delta.x.abs().max(delta.y.abs());
        let hops: f32 = (longest / hop_length).ceil().max(1.0);
        let hop = delta / hops;
        let hops: usize = hops.lossy_convert();

        let mut moving = hop;
        for _ in 0..hops {
            if moving.x != 0.0 && self.hop_x(&mut result, half, moving.x, moving.y > 0.0, drop_through) {
                moving.x = 0.0;
            }
            if moving.y != 0.0 && self.hop_y(&mut result, half, moving.y, drop_through) {
                moving.y = 0.0;
            }
            if moving.x == 0.0 && moving.y == 0.0 {
                break;
            }
        }

        result
    }

    /// Whether a box with this center and half size stands on ground, on
    /// a one way segment, or on a corner of the ground.
    pub fn stands_on_ground(&self, center: Point, half: Point) -> bool {
        let bottom = center.y - half.y;
        self.support(center.x, bottom - EDGE * 4.0, bottom + TOUCH, |_, _| true)
            .is_some()
            || self.sweep_down(center, half, EDGE * 4.0).is_some_and(|contact| contact.rests)
    }

    /// One hop along x, true when a wall stopped it.
    fn hop_x(&self, result: &mut BoxMove, half: Point, dx: f32, rising: bool, drop_through: bool) -> bool {
        let pos = result.position;
        let bottom = pos.y - half.y;
        let standing = self
            .support(pos.x, bottom - EDGE * 3.0, bottom + TOUCH, |_, _| !drop_through)
            .map(|(_, support)| support);

        let blocked = self.sweep_x(pos, half, dx);
        let moved = blocked.map_or(dx, |distance| distance * dx.signum());
        let mut next = Point::new(pos.x + moved, pos.y);

        // A walkable edge rises at most this much over the hop.
        let reach = moved.abs() * self.slope_radians().tan() + EDGE * 3.0;
        let down = if standing.is_some() && !rising { reach } else { 0.0 };
        let ground = self.support(next.x, bottom - down, bottom + reach, |index, y| {
            !drop_through && (y <= bottom + TOUCH || standing == Some(Support::OneWay(index)))
        });
        if let Some((ground, _)) = ground {
            let lift = ground + EDGE - bottom;
            if lift > 0.0 {
                if self.sweep_up(next, half, lift).is_some() {
                    result.wall = true;
                    return true;
                }
                next.y += lift;
            } else if lift < 0.0 {
                next.y -= self.sweep_down(next, half, -lift).map_or(-lift, |contact| contact.distance);
            }
        }

        result.position = next;
        result.wall |= blocked.is_some();
        blocked.is_some()
    }

    /// One hop along y, true when the ground or a ceiling stopped it.
    fn hop_y(&self, result: &mut BoxMove, half: Point, dy: f32, drop_through: bool) -> bool {
        if dy > 0.0 {
            if let Some(distance) = self.sweep_up(result.position, half, dy) {
                result.position.y += distance;
                result.ceiling = true;
                return true;
            }
            result.position.y += dy;
            return false;
        }
        self.fall(result, half, -dy, drop_through, true)
    }

    fn fall(&self, result: &mut BoxMove, half: Point, dist: f32, drop_through: bool, slide: bool) -> bool {
        let pos = result.position;
        let bottom = pos.y - half.y;
        let land = self
            .support(pos.x, bottom - dist - EDGE, bottom + TOUCH, |_, _| !drop_through)
            .map(|(ground, _)| bottom - ground - EDGE);
        let contact = self.sweep_down(pos, half, dist);

        if let Some(land) = land
            && contact.is_none_or(|contact| land <= contact.distance)
        {
            result.position.y -= land;
            result.floor = true;
            return true;
        }

        let Some(contact) = contact else {
            result.position.y -= dist;
            return false;
        };

        result.position.y -= contact.distance;
        if contact.rests {
            result.floor = true;
            return true;
        }
        if !slide {
            return false;
        }

        // Down a steep slope by the rest of the hop, which takes this much
        // sideways.
        let left = dist - contact.distance;
        let dx = contact.normal.x.signum() * left * (contact.normal.y / contact.normal.x).abs();
        let moved = self
            .sweep_x(result.position, half, dx)
            .map_or(dx, |distance| distance * dx.signum());
        if moved.abs() < TOUCH {
            result.floor = true;
            return true;
        }
        result.position.x += moved;
        self.fall(result, half, left, drop_through, false)
    }
}
