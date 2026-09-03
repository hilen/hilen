use crate::{
    deps::refs::Weak,
    gm::{
        ToF32,
        volume::{Quat, Ray, Vec3},
    },
    scene::Model,
};

/// How thick the slab under a plane is. Thin enough to read as a plane,
/// thick enough that a fast body cannot tunnel through it in one step.
pub(crate) const PLANE_THICKNESS: f32 = 0.2;

/// The solid of a node. Its mesh and its collider both come from it, so
/// what is drawn is what collides.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Shape3 {
    /// Full side lengths.
    Box(Vec3),
    Ball(f32),
    /// A square in the xz plane facing up, `side` wide and deep. The
    /// collider is a thin slab so bodies rest on it.
    Plane(f32),
    /// A loaded `.glb`, drawn at its own size with its own materials.
    /// The collider is the box around its bounds.
    Model(Weak<Model>),
}

impl Shape3 {
    pub fn cube(side: impl ToF32) -> Self {
        Self::Box(Vec3::splat(side.to_f32()))
    }

    pub fn cuboid(width: impl ToF32, height: impl ToF32, depth: impl ToF32) -> Self {
        Self::Box(Vec3::new(width.to_f32(), height.to_f32(), depth.to_f32()))
    }

    /// Half the size along each axis, what rapier wants.
    pub fn half_extents(&self) -> Vec3 {
        match self {
            Self::Box(size) => *size / 2.0,
            Self::Ball(radius) => Vec3::splat(*radius),
            Self::Plane(side) => Vec3::new(side / 2.0, PLANE_THICKNESS / 2.0, side / 2.0),
            Self::Model(model) => model.bounds.half_extents(),
        }
    }

    /// Where the collider sits relative to the node's origin. A plane is
    /// drawn at its origin and its slab hangs below it, so a body rests
    /// exactly on the drawn surface. A model's origin is wherever it was
    /// modeled, its box sits around its bounds.
    pub(crate) fn collider_offset(&self) -> Vec3 {
        match self {
            Self::Plane(_) => Vec3::new(0.0, -PLANE_THICKNESS / 2.0, 0.0),
            Self::Box(_) | Self::Ball(_) => Vec3::ZERO,
            Self::Model(model) => model.bounds.center(),
        }
    }

    /// The distance along the ray to this shape placed at `position`
    /// and turned by `rotation`. A ball by its surface, everything else
    /// by its collider box, so a model is hit on its bounds.
    pub fn hit(&self, ray: Ray, position: Vec3, rotation: Quat) -> Option<f32> {
        match self {
            Self::Ball(radius) => ray.hit_ball(position, *radius),
            Self::Box(_) | Self::Plane(_) | Self::Model(_) => ray.hit_box(
                position + rotation * self.collider_offset(),
                rotation,
                self.half_extents(),
            ),
        }
    }

    /// The scale that turns the unit mesh of this shape into its real
    /// size, see `Mesh::of_shape`.
    pub(crate) fn mesh_scale(&self) -> Vec3 {
        match self {
            Self::Box(size) => *size,
            Self::Ball(radius) => Vec3::splat(radius * 2.0),
            Self::Plane(side) => Vec3::new(*side, 1.0, *side),
            Self::Model(_) => Vec3::ONE,
        }
    }
}

impl Default for Shape3 {
    fn default() -> Self {
        Self::Box(Vec3::ONE)
    }
}
