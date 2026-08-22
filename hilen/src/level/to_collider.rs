use rapier2d::prelude::{ColliderBuilder, Vec2};

use crate::gm::{checked_usize_to_u32, flat::Shape};

pub trait ToCollider {
    fn make_collider(&self) -> ColliderBuilder;
}

impl ToCollider for Shape {
    fn make_collider(&self) -> ColliderBuilder {
        match self {
            Shape::Rect(size) => ColliderBuilder::cuboid(size.width / 2.0, size.height / 2.0),
            Shape::Circle(r) => ColliderBuilder::ball(*r),
            Shape::Triangle(a, b, c) => {
                ColliderBuilder::triangle([a.x, a.y].into(), [b.x, b.y].into(), [c.x, c.y].into())
            }
            Shape::Polygon(points) => convex_collider(points),
            Shape::Polyline(points) => polyline_collider(points),
        }
    }
}

fn make_indices(points: &[crate::gm::flat::Point]) -> (Vec<Vec2>, Vec<[u32; 2]>) {
    let points: Vec<_> = points.iter().map(|p| Vec2::new(p.x, p.y)).collect();
    let indices: Vec<_> = (0..u32::try_from(points.len()).unwrap() - 1)
        .map(|i| [i, i + 1])
        .chain([[checked_usize_to_u32(points.len()) - 1, 0]])
        .collect();
    (points, indices)
}

fn polyline_collider(points: &[crate::gm::flat::Point]) -> ColliderBuilder {
    let (points, indices) = make_indices(points);
    ColliderBuilder::polyline(points, Some(indices))
}

fn convex_collider(points: &[crate::gm::flat::Point]) -> ColliderBuilder {
    let (points, _indices) = make_indices(points);
    ColliderBuilder::convex_hull(&points).expect("Can't convex hull")
}
