use ui_proc::view;

use crate::{
    deps::refs::weak_from_ref,
    gm::{
        color::Color,
        flat::{FillRule, Paint, Point, StrokeStyle, VectorPath},
    },
    ui::{PathData, UIManager, View, ViewCallbacks, view::ViewFrame},
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
        let position = self.__base_view().draw_frame().origin;
        let resolution = UIManager::window_resolution();
        let scale = UIManager::scale() * self.__base_view().tree_scale;
        let z_position = self.z_position();
        let opacity = self.__base_view().tree_opacity;

        for path in &mut self.paths {
            path.prepare(position, resolution, scale, z_position, opacity);
        }
    }
}

impl DrawingView {
    pub(crate) fn paths(&self) -> &[PathData] {
        &self.paths
    }

    /// Strokes the path outline with a color or a gradient `Paint`.
    pub fn add_stroke(&self, path: &VectorPath, paint: impl Into<Paint>, style: StrokeStyle) -> &Self {
        let mut this = weak_from_ref(self);
        let (vertices, indices) = path.stroke_mesh(&style);
        this.push_mesh(&vertices, &indices, paint.into());
        self
    }

    /// Fills the path interior with a color or a gradient `Paint`.
    pub fn add_fill(&self, path: &VectorPath, paint: impl Into<Paint>, rule: FillRule) -> &Self {
        let mut this = weak_from_ref(self);
        let (vertices, indices) = path.fill_mesh(rule);
        this.push_mesh(&vertices, &indices, paint.into());
        self
    }

    /// Fills the closed polygon through the points.
    pub fn add_path<Container, P>(&self, path: Container, color: Color) -> &Self
    where
        P: Into<Point>,
        Container: IntoIterator<Item = P>, {
        self.add_fill(&VectorPath::polygon(path), color, FillRule::NonZero)
    }

    pub fn remove_all_paths(&self) {
        weak_from_ref(self).paths.clear();
    }

    fn push_mesh(&mut self, vertices: &[Point], indices: &[u32], paint: Paint) -> &mut Self {
        if indices.is_empty() {
            return self;
        }

        self.paths.push(PathData::new(paint, vertices, indices));
        self
    }
}
