use crate::{
    deps::refs::{Own, Weak},
    gm::{
        flat::Point,
        volume::{Ray, Shape3, Vec3},
    },
    scene::{Node, SceneBase},
};

/// Where a ray met a triangle of a model, see `SceneBase::hit_triangle`.
#[derive(Clone, Copy)]
pub struct TriangleHit {
    pub node:     Weak<dyn Node>,
    /// How far along the ray the triangle is.
    pub distance: f32,
    /// The point on the triangle in the world.
    pub point:    Vec3,
    /// The unit normal of the triangle in the world, on its drawn side.
    pub normal:   Vec3,
    /// Which triangle of the node's model, see `Model::triangle`. For a
    /// mesh built in code it is the index of the triangle in its
    /// `MeshData`, so an app finds its own data for it, like the part a
    /// CSG library made the triangle from.
    pub triangle: usize,
}

impl SceneBase {
    /// The nearest triangle of a model that `ray` meets on its drawn side,
    /// the exact surface where `node_at` and a touch know only the box
    /// around a model. Only nodes that draw a model take part, a model at
    /// rest, and the box around each is tried first, so a ray far from a
    /// big mesh costs little.
    pub fn hit_triangle(&self, ray: Ray) -> Option<TriangleHit> {
        self.nodes
            .iter()
            .filter_map(|node| node_hit(node, ray))
            .min_by(|a, b| a.distance.total_cmp(&b.distance))
    }

    /// `hit_triangle` for the ray through a pixel of the scene's area.
    pub fn triangle_at(&self, point: Point) -> Option<TriangleHit> {
        self.hit_triangle(self.ray(point))
    }
}

fn node_hit(node: &Own<dyn Node>, ray: Ray) -> Option<TriangleHit> {
    let Shape3::Model(model) = node.shape() else {
        return None;
    };
    if !model.is_ok() {
        return None;
    }
    node.hit(ray)?;

    // In the model's own space the direction keeps the node's scale, so
    // the distance along it is the one in the world.
    let matrix = node.model_matrix();
    let inverse = matrix.inverse();
    let local = Ray {
        origin:    inverse.transform_point3(ray.origin),
        direction: inverse.transform_vector3(ray.direction),
    };
    let (distance, triangle) = model.hit_triangle(local)?;
    let [a, b, c] = model.triangle(triangle);
    Some(TriangleHit {
        node: node.weak(),
        distance,
        point: ray.at(distance),
        normal: matrix.transform_vector3((b - a).cross(c - a)).normalize_or_zero(),
        triangle,
    })
}
