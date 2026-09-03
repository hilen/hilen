use ui_proc::view;

use crate::{
    gm::{
        color::Color,
        flat::{FillRule, Paint, Point, StrokeStyle, VectorPath},
    },
    render::data::PathData,
    ui::{UIManager, ViewCallbacks, view::ViewFrame},
};

/// Draws vector paths: stroked polylines and curves, filled shapes.
/// Path points are in this view's own coordinate space. Meshes are
/// tessellated once when a path is added, only the placement uniform
/// follows the view afterwards.
#[view]
pub struct DrawingView {
    paths: Vec<PathData>,
}

impl ViewCallbacks for DrawingView {
    fn update(&mut self) {
        let position = self.absolute_frame().origin;
        let resolution = UIManager::window_resolution();
        let scale = UIManager::scale();
        let z_position = self.z_position();

        for path in &mut self.paths {
            path.prepare(position, resolution, scale, z_position);
        }
    }
}

impl DrawingView {
    pub(crate) fn paths(&self) -> &[PathData] {
        &self.paths
    }

    /// Strokes the path outline with a color or a gradient `Paint`.
    pub fn add_stroke(
        &mut self,
        path: &VectorPath,
        paint: impl Into<Paint>,
        style: StrokeStyle,
    ) -> &mut Self {
        let (vertices, indices) = path.stroke_mesh(&style);
        self.push_mesh(&vertices, &indices, paint.into())
    }

    /// Fills the path interior with a color or a gradient `Paint`.
    pub fn add_fill(&mut self, path: &VectorPath, paint: impl Into<Paint>, rule: FillRule) -> &mut Self {
        let (vertices, indices) = path.fill_mesh(rule);
        self.push_mesh(&vertices, &indices, paint.into())
    }

    /// Fills the closed polygon through the points.
    pub fn add_path<Container, P>(&mut self, path: Container, color: Color) -> &mut Self
    where
        P: Into<Point>,
        Container: IntoIterator<Item = P>, {
        self.add_fill(&VectorPath::polygon(path), color, FillRule::NonZero)
    }

    pub fn remove_all_paths(&mut self) {
        self.paths.clear();
    }

    fn push_mesh(&mut self, vertices: &[Point], indices: &[u32], paint: Paint) -> &mut Self {
        if indices.is_empty() {
            return self;
        }

        self.paths.push(PathData::new(paint, vertices, indices));
        self
    }
}
