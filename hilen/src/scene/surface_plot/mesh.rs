use crate::{
    gm::{
        ToF32,
        color::Color,
        flat::Point,
        volume::{Vec3, Vertex3D},
    },
    scene::MeshData,
};

/// Half the side of the square the surface is drawn on, in meters.
pub(super) const WORLD_HALF: f32 = 5.0;
/// A value above or below this many meters is cut flat, so `x^9` does
/// not draw a wall a kilometer high.
const HEIGHT_LIMIT: f32 = 2.0 * WORLD_HALF;

// Low values blue, the middle pale, high values red, like a heat map.
const LOW: Color = Color::hex("#2C7BB6");
const MIDDLE: Color = Color::hex("#F7F4C8");
const HIGH: Color = Color::hex("#D7191C");

/// The surface of `z = function(x, y)` for x and y from `-half` to `half`
/// in `steps` cells a side. The function gives the height and maybe a
/// color, a point with no color of its own takes the color of its height. Math
/// z is up, so it is the y of the scene, and math y runs away from the camera,
/// the -z of the scene. A unit is as long on all 3 axes. Every triangle has a
/// back side too, since a scene culls back faces and a surface is seen from
/// both. A cell with a value that is not finite is left out.
pub(super) fn surface_mesh(
    function: &dyn Fn(f64, f64) -> (f64, Option<Color>),
    half: f64,
    steps: u32,
) -> MeshData {
    let side = steps + 1;
    let scale = f64::from(WORLD_HALF) / half;

    let mut positions = Vec::new();
    let mut finite = Vec::new();
    let mut colors = Vec::new();
    for row in 0..side {
        let y = -half + 2.0 * half * f64::from(row) / f64::from(steps);
        for column in 0..side {
            let x = -half + 2.0 * half * f64::from(column) / f64::from(steps);
            let (z, color) = function(x, y);
            finite.push(z.is_finite());
            colors.push(color);
            let height = if z.is_finite() {
                (z * scale).clamp(-f64::from(HEIGHT_LIMIT), f64::from(HEIGHT_LIMIT))
            } else {
                0.0
            };
            positions.push(Vec3::new(
                (x * scale).to_f32(),
                height.to_f32(),
                (-y * scale).to_f32(),
            ));
        }
    }

    let (low, high) = positions
        .iter()
        .zip(&finite)
        .filter(|(_, finite)| **finite)
        .fold((f32::MAX, f32::MIN), |(low, high), (position, _)| {
            (low.min(position.y), high.max(position.y))
        });

    let at = |row: u32, column: u32| (row * side + column) as usize;
    let mut front = Vec::new();
    for row in 0..side {
        for column in 0..side {
            let position = positions[at(row, column)];
            let along_x =
                positions[at(row, (column + 1).min(steps))] - positions[at(row, column.saturating_sub(1))];
            let along_y =
                positions[at((row + 1).min(steps), column)] - positions[at(row.saturating_sub(1), column)];
            let normal = along_x.cross(along_y).normalize_or(Vec3::Y);
            let mut vertex = Vertex3D::new(position, normal, Point::default());
            let color = colors[at(row, column)].unwrap_or_else(|| height_color(position.y, low, high));
            vertex.color = color.into();
            front.push(vertex);
        }
    }

    let count = u32::try_from(front.len()).expect("a surface fits u32 indices");
    let back: Vec<Vertex3D> = front
        .iter()
        .map(|vertex| Vertex3D {
            normal: -vertex.normal,
            ..*vertex
        })
        .collect();

    let mut indices = Vec::new();
    for row in 0..steps {
        for column in 0..steps {
            let a = row * side + column;
            let b = a + 1;
            let c = a + side;
            let d = c + 1;
            let corners = [a, b, c, d];
            if !corners.iter().all(|corner| finite[*corner as usize]) {
                continue;
            }
            indices.extend([a, b, c, b, d, c]);
            indices.extend([a + count, c + count, b + count, b + count, c + count, d + count]);
        }
    }

    let mut vertices = front;
    vertices.extend(back);
    MeshData { vertices, indices }
}

fn height_color(height: f32, low: f32, high: f32) -> Color {
    let share = if high > low {
        (height - low) / (high - low)
    } else {
        0.5
    };
    if share < 0.5 {
        mix(LOW, MIDDLE, share * 2.0)
    } else {
        mix(MIDDLE, HIGH, share * 2.0 - 1.0)
    }
}

fn mix(from: Color, to: Color, share: f32) -> Color {
    Color::rgba(
        from.r + (to.r - from.r) * share,
        from.g + (to.g - from.g) * share,
        from.b + (to.b - from.b) * share,
        1.0,
    )
}

#[cfg(test)]
mod test {
    use super::{WORLD_HALF, surface_mesh};
    use crate::gm::color::Color;

    #[test]
    fn a_flat_plane_has_both_sides() {
        let mesh = surface_mesh(&|_, _| (0.0, None), 1.0, 4);
        // 5 by 5 corners, twice for the back side.
        assert_eq!(mesh.vertices.len(), 50);
        // 16 cells, 2 triangles each, twice.
        assert_eq!(mesh.indices.len(), 16 * 2 * 3 * 2);
        assert!(mesh.vertices[..25].iter().all(|vertex| vertex.normal.y > 0.99));
        assert!(mesh.vertices[25..].iter().all(|vertex| vertex.normal.y < -0.99));
    }

    #[test]
    fn math_axes_map_to_the_scene() {
        let mesh = surface_mesh(&|x, y| (x + 2.0 * y, None), 2.0, 2);
        let scale = WORLD_HALF / 2.0;
        // The last corner is x 2, y 2, z 6, cut at the height limit.
        let corner = mesh.vertices[8].pos;
        assert!((corner.x - 2.0 * scale).abs() < 1e-5);
        assert!((corner.z + 2.0 * scale).abs() < 1e-5);
        assert!((corner.y - 2.0 * WORLD_HALF).abs() < 1e-5);
    }

    #[test]
    fn cells_that_are_not_finite_are_left_out() {
        let mesh = surface_mesh(&|x, _| (if x > 0.5 { f64::NAN } else { 0.0 }, None), 1.0, 2);
        // Only the 2 cells with x up to 0 stay.
        assert_eq!(mesh.indices.len(), 2 * 2 * 3 * 2);
    }

    #[test]
    fn a_given_color_wins_over_the_height() {
        let green = Color::rgba(0.0, 1.0, 0.0, 1.0);
        let mesh = surface_mesh(&|x, _| (x, if x < 0.0 { Some(green) } else { None }), 1.0, 2);
        let first = mesh.vertices[0].color;
        assert_eq!((first.r, first.g, first.b), (0, 255, 0));
        let last = mesh.vertices[8].color;
        assert_ne!((last.r, last.g, last.b), (0, 255, 0));
    }
}
