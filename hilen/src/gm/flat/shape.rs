use crate::gm::{
    ToF32,
    flat::{Point, ProcessPoints, Size},
};

#[derive(Clone, Debug)]
pub enum Shape {
    Rect(Size),
    Circle(f32),
    Triangle(Point, Point, Point),
    Polygon(Vec<Point>),
    Polyline(Vec<Point>),
}

impl Shape {
    pub fn rect(width: impl ToF32, height: impl ToF32) -> Self {
        Self::Rect((width, height).into())
    }

    pub fn triangle(a: impl Into<Point>, b: impl Into<Point>, c: impl Into<Point>) -> Self {
        Self::Triangle(a.into(), b.into(), c.into())
    }

    pub(crate) fn is_rect(&self) -> bool {
        matches!(self, Shape::Rect(_))
    }
}

impl Shape {
    pub fn size(&self) -> Size {
        match self {
            Self::Rect(size) => *size,
            Self::Circle(r) => (*r, *r).into(),
            Self::Triangle(a, b, c) => vec![*a, *b, *c].size() / 2.0,
            Self::Polygon(points) | Self::Polyline(points) => points.size() / 2.0,
        }
    }

    pub fn width(&self) -> f32 {
        match self {
            Self::Rect(size) => size.width,
            Self::Circle(r) => *r,
            Self::Triangle(a, b, c) => vec![*a, *b, *c].width() / 2.0,
            Self::Polygon(points) | Self::Polyline(points) => points.width() / 2.0,
        }
    }

    pub fn height(&self) -> f32 {
        match self {
            Self::Rect(size) => size.height,
            Self::Circle(r) => *r,
            Self::Triangle(a, b, c) => vec![*a, *b, *c].height() / 2.0,
            Self::Polygon(points) | Self::Polyline(points) => points.height() / 2.0,
        }
    }
}

impl Default for Shape {
    fn default() -> Self {
        Shape::Rect(Size::default())
    }
}
