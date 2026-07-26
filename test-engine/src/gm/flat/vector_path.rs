use log::error;
use lyon::{
    math::point,
    path::{Path, Winding},
    tessellation::{
        BuffersBuilder, FillOptions, FillTessellator, FillVertex, StrokeOptions, StrokeTessellator,
        StrokeVertex, VertexBuffers,
    },
};

use crate::gm::{flat::Point, num::into_f32::ToF32};

/// The shape of a stroke's ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LineCap {
    #[default]
    Butt,
    Square,
    Round,
}

/// The shape of a stroke's corners.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LineJoin {
    #[default]
    Miter,
    Bevel,
    Round,
}

/// How a fill decides what is inside a self intersecting path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FillRule {
    #[default]
    NonZero,
    EvenOdd,
}

/// How a path outline is stroked.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StrokeStyle {
    pub width:       f32,
    pub cap:         LineCap,
    pub join:        LineJoin,
    pub miter_limit: f32,
}

impl Default for StrokeStyle {
    fn default() -> Self {
        Self {
            width:       1.0,
            cap:         LineCap::default(),
            join:        LineJoin::default(),
            miter_limit: StrokeOptions::DEFAULT_MITER_LIMIT,
        }
    }
}

impl StrokeStyle {
    pub fn width(width: impl ToF32) -> Self {
        Self {
            width: width.to_f32(),
            ..Self::default()
        }
    }

    pub fn cap(mut self, cap: LineCap) -> Self {
        self.cap = cap;
        self
    }

    pub fn join(mut self, join: LineJoin) -> Self {
        self.join = join;
        self
    }

    pub fn miter_limit(mut self, limit: impl ToF32) -> Self {
        self.miter_limit = limit.to_f32();
        self
    }
}

/// Vector path geometry: lines and bezier curves in the owning view's
/// coordinate space. Color and stroke or fill style are chosen when the
/// path is added to a `DrawingView`.
pub struct VectorPath {
    path: Path,
}

impl VectorPath {
    pub fn builder() -> VectorPathBuilder {
        VectorPathBuilder {
            builder: Path::builder(),
            started: false,
        }
    }

    /// An open polyline through the points.
    pub fn polyline<P: Into<Point>>(points: impl IntoIterator<Item = P>) -> Self {
        Self::from_points(points, false)
    }

    /// A closed polygon through the points.
    pub fn polygon<P: Into<Point>>(points: impl IntoIterator<Item = P>) -> Self {
        Self::from_points(points, true)
    }

    pub fn circle(center: impl Into<Point>, radius: impl ToF32) -> Self {
        let center = center.into();
        let mut builder = Path::builder();
        builder.add_circle(point(center.x, center.y), radius.to_f32(), Winding::Positive);
        Self {
            path: builder.build(),
        }
    }

    fn from_points<P: Into<Point>>(points: impl IntoIterator<Item = P>, close: bool) -> Self {
        let mut builder = Path::builder();
        let mut started = false;

        for p in points {
            let p = p.into();
            if started {
                builder.line_to(point(p.x, p.y));
            } else {
                builder.begin(point(p.x, p.y));
                started = true;
            }
        }

        if started {
            builder.end(close);
        }

        Self {
            path: builder.build(),
        }
    }

    pub fn stroke_mesh(&self, style: &StrokeStyle) -> (Vec<Point>, Vec<u32>) {
        let options = StrokeOptions::default()
            .with_line_width(style.width)
            .with_start_cap(cap(style.cap))
            .with_end_cap(cap(style.cap))
            .with_line_join(join(style.join))
            .with_miter_limit(style.miter_limit);

        let mut geometry: VertexBuffers<Point, u32> = VertexBuffers::new();

        let tessellated = StrokeTessellator::new().tessellate_path(
            &self.path,
            &options,
            &mut BuffersBuilder::new(&mut geometry, |vertex: StrokeVertex| Point {
                x: vertex.position().x,
                y: vertex.position().y,
            }),
        );

        // Only degenerate input fails, NaN coordinates or a miter limit
        // below 1. Drawing nothing beats crashing an app on bad data.
        if let Err(err) = tessellated {
            error!("Stroke tessellation failed: {err:?}");
            return (vec![], vec![]);
        }

        (geometry.vertices, geometry.indices)
    }

    pub fn fill_mesh(&self, rule: FillRule) -> (Vec<Point>, Vec<u32>) {
        let options = FillOptions::default().with_fill_rule(match rule {
            FillRule::NonZero => lyon::tessellation::FillRule::NonZero,
            FillRule::EvenOdd => lyon::tessellation::FillRule::EvenOdd,
        });

        let mut geometry: VertexBuffers<Point, u32> = VertexBuffers::new();

        let tessellated = FillTessellator::new().tessellate_path(
            &self.path,
            &options,
            &mut BuffersBuilder::new(&mut geometry, |vertex: FillVertex| Point {
                x: vertex.position().x,
                y: vertex.position().y,
            }),
        );

        if let Err(err) = tessellated {
            error!("Fill tessellation failed: {err:?}");
            return (vec![], vec![]);
        }

        (geometry.vertices, geometry.indices)
    }
}

/// Canvas style path builder. `line_to` before any `move_to` starts the
/// sub path at that point.
pub struct VectorPathBuilder {
    builder: lyon::path::path::Builder,
    started: bool,
}

impl VectorPathBuilder {
    pub fn move_to(mut self, to: impl Into<Point>) -> Self {
        if self.started {
            self.builder.end(false);
        }
        let to = to.into();
        self.builder.begin(point(to.x, to.y));
        self.started = true;
        self
    }

    /// Adds a full circle as its own sub path. With `FillRule::EvenOdd`
    /// a circle inside another shape cuts a hole, the donut case.
    pub fn circle(mut self, center: impl Into<Point>, radius: impl ToF32) -> Self {
        if self.started {
            self.builder.end(false);
            self.started = false;
        }
        let center = center.into();
        self.builder
            .add_circle(point(center.x, center.y), radius.to_f32(), Winding::Positive);
        self
    }

    pub fn line_to(mut self, to: impl Into<Point>) -> Self {
        if !self.started {
            return self.move_to(to);
        }
        let to = to.into();
        self.builder.line_to(point(to.x, to.y));
        self
    }

    pub fn quadratic_to(mut self, ctrl: impl Into<Point>, to: impl Into<Point>) -> Self {
        if !self.started {
            return self.move_to(to);
        }
        let ctrl = ctrl.into();
        let to = to.into();
        self.builder.quadratic_bezier_to(point(ctrl.x, ctrl.y), point(to.x, to.y));
        self
    }

    pub fn cubic_to(
        mut self,
        ctrl1: impl Into<Point>,
        ctrl2: impl Into<Point>,
        to: impl Into<Point>,
    ) -> Self {
        if !self.started {
            return self.move_to(to);
        }
        let ctrl1 = ctrl1.into();
        let ctrl2 = ctrl2.into();
        let to = to.into();
        self.builder.cubic_bezier_to(
            point(ctrl1.x, ctrl1.y),
            point(ctrl2.x, ctrl2.y),
            point(to.x, to.y),
        );
        self
    }

    pub fn close(mut self) -> Self {
        if self.started {
            self.builder.close();
            self.started = false;
        }
        self
    }

    pub fn build(mut self) -> VectorPath {
        if self.started {
            self.builder.end(false);
        }
        VectorPath {
            path: self.builder.build(),
        }
    }
}

fn cap(cap: LineCap) -> lyon::path::LineCap {
    match cap {
        LineCap::Butt => lyon::path::LineCap::Butt,
        LineCap::Square => lyon::path::LineCap::Square,
        LineCap::Round => lyon::path::LineCap::Round,
    }
}

fn join(join: LineJoin) -> lyon::path::LineJoin {
    match join {
        LineJoin::Miter => lyon::path::LineJoin::Miter,
        LineJoin::Bevel => lyon::path::LineJoin::Bevel,
        LineJoin::Round => lyon::path::LineJoin::Round,
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn polyline_stroke() {
        let path = VectorPath::polyline([(0, 0), (50, 50), (100, 0)]);
        let (vertices, indices) = path.stroke_mesh(&StrokeStyle::width(4));
        assert!(!vertices.is_empty());
        assert_eq!(indices.len() % 3, 0);
    }

    #[test]
    fn polygon_fill() {
        let concave = [(0, 0), (100, 10), (100, 100), (50, 40), (0, 100)];
        let (vertices, indices) = VectorPath::polygon(concave).fill_mesh(FillRule::NonZero);
        assert!(!vertices.is_empty());
        assert_eq!(indices.len() % 3, 0);
    }

    #[test]
    fn circle_fill() {
        let (vertices, indices) = VectorPath::circle((50, 50), 25).fill_mesh(FillRule::NonZero);
        assert!(!vertices.is_empty());
        assert_eq!(indices.len() % 3, 0);
    }

    #[test]
    fn builder_curves() {
        let path = VectorPath::builder()
            .move_to((0, 0))
            .line_to((20, 0))
            .quadratic_to((30, 10), (20, 20))
            .cubic_to((10, 30), (5, 30), (0, 20))
            .close()
            .build();

        let (vertices, indices) = path.fill_mesh(FillRule::NonZero);
        assert!(!vertices.is_empty());
        assert_eq!(indices.len() % 3, 0);
    }

    #[test]
    fn donut_has_a_hole() {
        let ring = VectorPath::builder().circle((50, 50), 40).circle((50, 50), 20).build();

        let (vertices, indices) = ring.fill_mesh(FillRule::EvenOdd);
        assert!(!vertices.is_empty());
        assert_eq!(indices.len() % 3, 0);

        // Every triangle must stay outside the inner circle, so no
        // triangle center may come closer than the inner radius.
        for triangle in indices.chunks(3) {
            let center_x = triangle.iter().map(|i| vertices[*i as usize].x).sum::<f32>() / 3.0;
            let center_y = triangle.iter().map(|i| vertices[*i as usize].y).sum::<f32>() / 3.0;
            let distance = ((center_x - 50.0).powi(2) + (center_y - 50.0).powi(2)).sqrt();
            assert!(distance > 19.0, "triangle center inside the hole: {distance}");
        }
    }

    #[test]
    fn empty_path_yields_nothing() {
        let none: [Point; 0] = [];
        let (vertices, indices) = VectorPath::polyline(none).stroke_mesh(&StrokeStyle::default());
        assert!(vertices.is_empty());
        assert!(indices.is_empty());
    }
}
