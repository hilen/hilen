use std::{
    fmt::Display,
    hash::{Hash, Hasher},
    ops::{Div, Mul},
};

use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};

use crate::gm::{
    flat::Point,
    num::{IsZero, into_f32::ToF32, lossy_convert::LossyConvert},
};

#[derive(Copy, Clone, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Size<T = f32> {
    pub width:  T,
    pub height: T,
}

unsafe impl<T: Zeroable> Zeroable for Size<T> {}

unsafe impl<T: Pod> Pod for Size<T> {}

impl<T> Size<T> {
    pub const fn new(width: T, height: T) -> Self {
        Self { width, height }
    }

    pub(crate) fn into_point(self) -> Point<T> {
        Point::new(self.width, self.height)
    }
}

impl<T: Copy + PartialOrd> Size<T> {
    pub(crate) fn smallest_side(&self) -> T {
        if self.width < self.height {
            self.width
        } else {
            self.height
        }
    }
}

impl<T: Copy> Size<T> {
    pub const fn as_slice(&self) -> [T; 2] {
        [self.width, self.height]
    }
}

impl<T: IsZero> Size<T> {
    pub(crate) fn has_no_area(&self) -> bool {
        self.width.is_zero() || self.height.is_zero()
    }
}

impl<T: Mul<Output = T> + Copy> Size<T> {
    pub fn area(&self) -> T {
        self.width.mul(self.height)
    }
}

impl<T: LossyConvert<U>, U: Copy> LossyConvert<Size<U>> for Size<T> {
    fn lossy_convert(self) -> Size<U> {
        Size::new(self.width.lossy_convert(), self.height.lossy_convert())
    }
}

impl Size<f32> {
    pub fn diagonal(&self) -> f32 {
        (self.width * self.width + self.height * self.height).sqrt()
    }

    pub fn is_negative(&self) -> bool {
        self.width < 0.0 || self.height < 0.0
    }

    pub fn center(&self) -> Point {
        Point {
            x: self.width / 2.0,
            y: self.height / 2.0,
        }
    }

    pub fn fit_height(&self, height: impl ToF32) -> Size {
        let ratio = height.to_f32() / self.height;
        *self * ratio
    }

    pub fn fit_width(&self, width: impl ToF32) -> Size {
        let ratio = width.to_f32() / self.width;
        *self * ratio
    }
}

impl<W: ToF32, H: ToF32> From<(W, H)> for Size<f32> {
    fn from(tup: (W, H)) -> Self {
        Self {
            width:  tup.0.to_f32(),
            height: tup.1.to_f32(),
        }
    }
}

impl<T: ToF32> Mul<T> for Size<f32> {
    type Output = Size;
    fn mul(self, rhs: T) -> Self::Output {
        (self.width * rhs.to_f32(), self.height * rhs.to_f32()).into()
    }
}

impl<T: ToF32> Div<T> for Size<f32> {
    type Output = Self;
    fn div(self, rhs: T) -> Self::Output {
        (self.width / rhs.to_f32(), self.height / rhs.to_f32()).into()
    }
}

impl Hash for Size<f32> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.width.to_bits().hash(state);
        self.height.to_bits().hash(state);
    }
}

impl From<Size<u32>> for Size<f32> {
    fn from(value: Size<u32>) -> Self {
        Self {
            width:  value.width.lossy_convert(),
            height: value.height.lossy_convert(),
        }
    }
}

impl From<(u32, u32)> for Size<u32> {
    fn from(tup: (u32, u32)) -> Self {
        Self {
            width:  tup.0,
            height: tup.1,
        }
    }
}

impl Display for Size {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "width: {:.2}, height: {:.2}", self.width, self.height)
    }
}

impl Display for Size<u32> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "width: {}, height: {}", self.width, self.height)
    }
}
