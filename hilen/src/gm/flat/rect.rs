use std::ops::Mul;

use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};

use crate::gm::{
    Zero,
    axis::Axis,
    flat::{Point, Size},
    num::into_f32::ToF32,
};

#[repr(C)]
#[derive(Copy, Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rect<T = f32> {
    pub origin: Point<T>,
    pub size:   Size<T>,
}

unsafe impl<T: Zeroable> Zeroable for Rect<T> {}
unsafe impl<T: Pod> Pod for Rect<T> {}

impl<T> Rect<T> {
    pub const fn new(x: T, y: T, w: T, h: T) -> Self {
        Self {
            origin: Point::new(x, y),
            size:   Size::new(w, h),
        }
    }
}

impl<T: Copy> Rect<T> {
    pub fn x(&self) -> T {
        self.origin.x
    }

    pub fn y(&self) -> T {
        self.origin.y
    }

    pub fn width(&self) -> T {
        self.size.width
    }

    pub fn height(&self) -> T {
        self.size.height
    }
}

impl Rect<f32> {
    pub fn max_x(&self) -> f32 {
        self.x() + self.width()
    }

    pub fn max_y(&self) -> f32 {
        self.y() + self.height()
    }

    pub fn contains(&self, point: impl Into<Point>) -> bool {
        let point = point.into();
        point.x >= self.x()
            && point.y >= self.y()
            && point.x <= self.x() + self.width()
            && point.y <= self.y() + self.height()
    }

    pub(crate) fn intersects(&self, rect: &Rect) -> bool {
        let x1 = self.x();
        let y1 = self.y();
        let x2 = x1 + self.width();
        let y2 = y1 + self.height();

        let x3 = rect.x();
        let y3 = rect.y();
        let x4 = x3 + rect.width();
        let y4 = y3 + rect.height();

        x1 < x4 && x2 > x3 && y1 < y4 && y2 > y3
    }

    pub fn square(&self) -> Size {
        let side = if self.height() < self.width() {
            self.height()
        } else {
            self.width()
        };
        (side, side).into()
    }

    pub(crate) fn fit_aspect_ratio(&self, size: Size) -> Rect {
        let scale_x = self.size.width / size.width;
        let scale_y = self.size.height / size.height;
        let scale = scale_x.min(scale_y);

        let new_width = size.width * scale;
        let new_height = size.height * scale;

        let x = self.origin.x + (self.size.width - new_width) / 2.0;
        let y = self.origin.y + (self.size.height - new_height) / 2.0;

        Rect {
            origin: Point { x, y },
            size:   Size {
                width:  new_width,
                height: new_height,
            },
        }
    }
}

impl Rect<u32> {
    pub(crate) fn intersection(&self, other: &Self) -> Self {
        let x = self.x().max(other.x());
        let y = self.y().max(other.y());
        let max_x = (self.x() + self.width()).min(other.x() + other.width());
        let max_y = (self.y() + self.height()).min(other.y() + other.height());
        Self::new(x, y, max_x.saturating_sub(x), max_y.saturating_sub(y))
    }
}

impl Rect {
    pub fn center(&self) -> Point {
        (self.x() + self.width() / 2.0, self.y() + self.height() / 2.0).into()
    }

    pub(crate) fn set_center(&mut self, center: impl Into<Point>) {
        let center = center.into();
        self.origin.x = center.x - self.width() / 2.0;
        self.origin.y = center.y - self.height() / 2.0;
    }

    pub(crate) fn with_zero_origin(&self) -> Rect {
        (0, 0, self.size.width, self.size.height).into()
    }

    pub(crate) fn to_borders(self, width: impl ToF32) -> [Rect; 4] {
        let width = width.to_f32();

        [
            (self.x(), self.y(), self.width(), width).into(),
            (self.x() + self.width() - width, self.y(), width, self.height()).into(),
            (self.x(), self.y() + self.height() - width, self.width(), width).into(),
            (self.x(), self.y(), width, self.height()).into(),
        ]
    }
}

impl Rect {
    pub(crate) fn length<const AXIS: Axis>(&self) -> f32 {
        match AXIS {
            Axis::X => self.size.width,
            Axis::Y => self.size.height,
        }
    }

    pub(crate) fn other_length<const AXIS: Axis>(&self) -> f32 {
        match AXIS {
            Axis::X => self.size.height,
            Axis::Y => self.size.width,
        }
    }

    pub(crate) fn set_position<const AXIS: Axis>(&mut self, pos: impl ToF32) {
        match AXIS {
            Axis::X => self.origin.x = pos.to_f32(),
            Axis::Y => self.origin.y = pos.to_f32(),
        }
    }

    pub(crate) fn set_other_position<const AXIS: Axis>(&mut self, pos: impl ToF32) {
        match AXIS {
            Axis::X => self.origin.y = pos.to_f32(),
            Axis::Y => self.origin.x = pos.to_f32(),
        }
    }

    pub(crate) fn set_length<const AXIS: Axis>(&mut self, length: impl ToF32) {
        match AXIS {
            Axis::X => self.size.width = length.to_f32(),
            Axis::Y => self.size.height = length.to_f32(),
        }
    }

    pub(crate) fn set_other_length<const AXIS: Axis>(&mut self, length: impl ToF32) {
        match AXIS {
            Axis::X => self.size.height = length.to_f32(),
            Axis::Y => self.size.width = length.to_f32(),
        }
    }
}

impl<T: Zero> From<Size<T>> for Rect<T> {
    fn from(size: Size<T>) -> Self {
        Rect {
            origin: Point {
                x: T::zero(),
                y: T::zero(),
            },
            size,
        }
    }
}

impl<X, Y, W, H> From<(X, Y, W, H)> for Rect
where
    X: ToF32,
    Y: ToF32,
    W: ToF32,
    H: ToF32,
{
    fn from(tup: (X, Y, W, H)) -> Self {
        Self {
            origin: Point {
                x: tup.0.to_f32(),
                y: tup.1.to_f32(),
            },
            size:   Size {
                width:  tup.2.to_f32(),
                height: tup.3.to_f32(),
            },
        }
    }
}

impl<T: ToF32> Mul<T> for &Rect {
    type Output = Rect;
    fn mul(self, rhs: T) -> Rect {
        let mul = rhs.to_f32();
        (
            self.origin.x * mul,
            self.origin.y * mul,
            self.size.width * mul,
            self.size.height * mul,
        )
            .into()
    }
}

impl<T: ToF32> Mul<T> for Rect<f32> {
    type Output = Rect<f32>;
    fn mul(self, rhs: T) -> Rect<f32> {
        (&self).mul(rhs)
    }
}
