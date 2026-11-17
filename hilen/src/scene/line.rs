use crate::gm::{
    LossyConvert, ToF32,
    color::Color,
    volume::{Mat4, Vec3},
};

/// The axis colors of `Line3::axes`, x red, y green and z blue, the way
/// Blender and most 3D tools draw them.
pub const AXIS_COLORS: [Color; 3] = [
    Color::hex("#e5484d"),
    Color::hex("#46a758"),
    Color::hex("#3e8ed0"),
];

/// A straight line in a scene, `width` pixels wide on the screen at any
/// distance. A scene draws the lines in `SceneBase::lines` in the world
/// and a node the ones in its own `lines` in its own space, so they move,
/// turn and grow with it. A line is hidden behind the nodes in front of
/// it, or drawn over everything when `on_top` is set. It sits a thousandth
/// of its distance nearer to the eye, so the edge of a face drawn on that
/// face is not cut by it. Nothing lights a line.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Line3 {
    pub from:   Vec3,
    pub to:     Vec3,
    pub color:  Color,
    /// Pixels on the screen.
    pub width:  f32,
    pub on_top: bool,
}

impl Line3 {
    /// A line 1 pixel wide, hidden behind what is in front of it.
    pub fn new(from: impl Into<Vec3>, to: impl Into<Vec3>, color: impl Into<Color>) -> Self {
        Self {
            from:   from.into(),
            to:     to.into(),
            color:  color.into(),
            width:  1.0,
            on_top: false,
        }
    }

    #[must_use]
    pub fn width(mut self, width: impl ToF32) -> Self {
        self.width = width.to_f32();
        self
    }

    /// Drawn over every node, the way an outline of a selected part or a
    /// handle stays in sight.
    #[must_use]
    pub fn on_top(mut self) -> Self {
        self.on_top = true;
        self
    }

    /// The line moved into the space `matrix` maps to, where a node puts
    /// the lines of its own space.
    #[must_use]
    pub fn transformed(mut self, matrix: Mat4) -> Self {
        self.from = matrix.transform_point3(self.from);
        self.to = matrix.transform_point3(self.to);
        self
    }

    /// A square floor grid on the xz plane around the origin, `size`
    /// wide, a line every `step`.
    pub fn grid(size: impl ToF32, step: impl ToF32, color: impl Into<Color>) -> Vec<Self> {
        let size = size.to_f32();
        let step = step.to_f32();
        let color = color.into();
        assert!(step > 0.0, "a grid needs a step above 0, got {step}");
        let half = size / 2.0;
        let lines: u32 = (size / step).floor().lossy_convert();
        (0..=lines)
            .flat_map(|index| {
                let index: f32 = index.lossy_convert();
                let at = -half + index * step;
                [
                    Self::new((at, 0.0, -half), (at, 0.0, half), color),
                    Self::new((-half, 0.0, at), (half, 0.0, at), color),
                ]
            })
            .collect()
    }

    /// The x, y and z axes from the origin, each `length` long in its
    /// color of `AXIS_COLORS`.
    pub fn axes(length: impl ToF32) -> Vec<Self> {
        let length = length.to_f32();
        [Vec3::X, Vec3::Y, Vec3::Z]
            .into_iter()
            .zip(AXIS_COLORS)
            .map(|(axis, color)| Self::new(Vec3::ZERO, axis * length, color))
            .collect()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::gm::volume::Quat;

    #[test]
    fn a_grid_has_a_line_per_step_both_ways_and_ends_at_its_edge() {
        let grid = Line3::grid(4.0, 1.0, Color::hex("#808080"));
        assert_eq!(grid.len(), 10);
        assert!(grid.iter().all(|line| line.from.y == 0.0 && line.to.y == 0.0));
        assert!(
            grid.iter()
                .all(|line| line.from.abs().max_element() <= 2.0 && line.to.abs().max_element() <= 2.0)
        );
        assert_eq!(grid[0].from, Vec3::new(-2.0, 0.0, -2.0));
        assert_eq!(grid[9].to, Vec3::new(2.0, 0.0, 2.0));
    }

    #[test]
    fn a_line_follows_the_space_it_is_put_in() {
        let line = Line3::new(Vec3::ZERO, Vec3::X, Color::hex("#ffffff")).width(3).on_top();
        let matrix = Mat4::from_scale_rotation_translation(
            Vec3::splat(2.0),
            Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
            Vec3::new(1.0, 2.0, 3.0),
        );
        let moved = line.transformed(matrix);
        assert_eq!(moved.from, Vec3::new(1.0, 2.0, 3.0));
        assert!((moved.to - Vec3::new(1.0, 2.0, 1.0)).length() < 1e-5);
        assert!(moved.on_top && (moved.width - 3.0).abs() < f32::EPSILON);
    }
}
