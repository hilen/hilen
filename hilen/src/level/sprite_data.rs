use std::f32::consts::TAU;

use educe::Educe;

use crate::{
    deps::{refs::Weak, vents::Event},
    gm::{
        LossyConvert,
        color::Color,
        flat::{Point, PointsPath, Shape, Size},
    },
    level::Sprite,
    window::{VertexBuffer, image::Image},
};

const CIRCLE_POINTS: usize = 32;

#[derive(Educe)]
#[educe(Default)]
pub struct SpriteData {
    pub(crate) position: Point,

    pub(crate) size:        Size,
    pub(crate) render_size: Size,
    pub(crate) rotation:    f32,
    pub(crate) is_selected: bool,

    pub(crate) collision_enabled: bool,

    pub tag: u32,

    #[educe(Default = Color::random())]
    pub color: Color,

    #[educe(Default = 0.85)]
    pub z_position: f32,

    #[educe(Default = 1.0)]
    pub image_scale:  f32,
    pub image:        Weak<Image>,
    pub on_collision: Event<Weak<dyn Sprite>>,

    pub vertex_buffer: Option<VertexBuffer>,
}

impl SpriteData {
    pub(crate) fn make(shape: Shape, position: Point) -> Self {
        Self {
            position,
            size: shape.size(),
            // Rapier rect size defined by half side
            render_size: if shape.is_rect() {
                shape.size() / 2.0
            } else {
                shape.size()
            },
            vertex_buffer: Self::shape_to_buffer(shape),
            ..Default::default()
        }
    }
}

impl SpriteData {
    fn shape_to_buffer(shape: Shape) -> Option<VertexBuffer> {
        match shape {
            Shape::Rect(_) => None,
            // The sprite drawer only knows quads, so a circle is drawn as
            // a polygon. The collider stays a real ball.
            Shape::Circle(radius) => {
                let points: Vec<Point> = (0..CIRCLE_POINTS)
                    .map(|i| {
                        let angle = TAU * i.lossy_convert() / CIRCLE_POINTS.lossy_convert();
                        Point::new(radius * angle.cos(), radius * angle.sin())
                    })
                    .collect();
                let (vertices, indices) = PointsPath::tessellate(points);
                VertexBuffer {
                    vertices,
                    indices: indices.into(),
                }
                .into()
            }
            Shape::Triangle(a, b, c) => Some(vec![a, b, c].into()),
            Shape::Polygon(points) | Shape::Polyline(points) => {
                let (vertices, indices) = PointsPath::tessellate(points);
                VertexBuffer {
                    vertices,
                    indices: indices.into(),
                }
                .into()
            }
        }
    }
}
