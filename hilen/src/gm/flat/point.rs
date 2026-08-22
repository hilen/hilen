use std::{
    fmt::Display,
    ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign},
};

use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};

use crate::gm::{
    flat::Size,
    num::{into_f32::ToF32, lossy_convert::LossyConvert},
};

#[derive(Copy, Debug, Clone)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    pub fn is_up(&self) -> bool {
        matches!(self, Self::Up)
    }
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Point<T = f32> {
    pub x: T,
    pub y: T,
}

unsafe impl<T: Zeroable> Zeroable for Point<T> {}
unsafe impl<T: Pod> Pod for Point<T> {}

impl<T> Point<T> {
    pub const fn new(x: T, y: T) -> Self {
        Self { x, y }
    }

    pub(crate) fn into_size(self) -> Size<T> {
        Size::new(self.x, self.y)
    }
}

impl<T: LossyConvert<U>, U> LossyConvert<Point<U>> for Point<T> {
    fn lossy_convert(self) -> Point<U> {
        Point::new(self.x.lossy_convert(), self.y.lossy_convert())
    }
}

impl<T: PartialOrd> Point<T> {
    pub(crate) fn clamp(self, x: T, y: T) -> Self {
        let x = if x > self.x { x } else { self.x };
        let y = if y > self.y { y } else { self.y };
        Self::new(x, y)
    }
}

impl Point<f32> {
    pub(crate) fn angle(self) -> f32 {
        self.y.atan2(self.x)
    }

    pub(crate) fn angle_to(self, point: Self) -> f32 {
        let target = point - self;
        target.angle()
    }

    pub(crate) fn length(self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    pub(crate) fn middle(self, other: Self) -> Self {
        Point {
            x: f32::midpoint(self.x, other.x),
            y: f32::midpoint(self.y, other.y),
        }
    }

    pub(crate) fn is_positive(self) -> bool {
        self.x >= 0.0 && self.y >= 0.0
    }

    pub(crate) fn is_negative(self) -> bool {
        !self.is_positive()
    }

    pub(crate) fn neg(self) -> Self {
        (-self.x, -self.y).into()
    }

    pub(crate) fn clip_positive(&mut self) {
        self.x = self.x.max(0.0);
        self.y = self.y.max(0.0);
    }
}

impl Point<f32> {
    pub fn invert(&mut self) {
        self.x = -self.x;
        self.y = -self.y;
    }

    pub fn invert_x(&mut self) {
        self.x = -self.x;
    }

    pub fn invert_y(mut self) -> Self {
        self.y = -self.y;
        self
    }
}

impl Point<f32> {
    pub(crate) fn normalized(self) -> Self {
        self.with_length(1.0)
    }

    pub fn normalize(&mut self) {
        self.set_length(1.0);
    }
}

impl Point<f32> {
    pub(crate) fn with_length(self, l: f32) -> Self {
        let ratio = l / self.length();
        Point {
            x: self.x * ratio,
            y: self.y * ratio,
        }
    }

    pub(crate) fn set_length(&mut self, l: f32) {
        let ratio = l / self.length();
        self.x *= ratio;
        self.y *= ratio;
    }

    pub(crate) fn trim(&mut self, max_length: f32) {
        if self.length() < max_length {
            return;
        }
        self.set_length(max_length);
    }

    pub(crate) fn trimmed(mut self, max_length: f32) -> Self {
        self.trim(max_length);
        self
    }
}

impl Point<i32> {
    pub fn is_negative(&self) -> bool {
        self.x < 0 || self.y < 0
    }
}

impl<T: ToF32> Add<T> for Point<f32> {
    type Output = Self;
    fn add(self, rhs: T) -> Self::Output {
        Self::new(self.x + rhs.to_f32(), self.y + rhs.to_f32())
    }
}

impl<T: Add<Output = T>> Add for Point<T> {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl<T: Add<Output = T> + Copy> Add for &Point<T> {
    type Output = Point<T>;
    fn add(self, rhs: &Self::Output) -> Self::Output {
        Self::Output {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl<T: AddAssign> AddAssign for Point<T> {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl<T: Sub<Output = T>> Sub for Point<T> {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl<T: Sub<Output = T> + Copy> Sub for &Point<T> {
    type Output = Point<T>;
    fn sub(self, rhs: &Self::Output) -> Self::Output {
        Self::Output {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl<T: Sub<Output = T> + Copy> Sub<T> for Point<T> {
    type Output = Self;
    fn sub(self, rhs: T) -> Self::Output {
        Self::Output {
            x: self.x - rhs,
            y: self.y - rhs,
        }
    }
}

impl<T: SubAssign<T> + Copy> SubAssign<T> for Point<T> {
    fn sub_assign(&mut self, rhs: T) {
        self.x -= rhs;
        self.y -= rhs;
    }
}

impl<T: SubAssign> SubAssign for Point<T> {
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

impl<T: Mul<Output = T> + Copy> Mul<T> for Point<T> {
    type Output = Self;
    fn mul(self, rhs: T) -> Self {
        Self::new(self.x * rhs, self.y * rhs)
    }
}

impl Mul<Size> for Point<f32> {
    type Output = Self;
    fn mul(self, size: Size) -> Self {
        (self.x * size.width, self.y * size.height).into()
    }
}

impl<T: ToF32> MulAssign<T> for Point<f32> {
    fn mul_assign(&mut self, rhs: T) {
        let rhs = rhs.to_f32();
        self.x *= rhs;
        self.y *= rhs;
    }
}

impl<T: Div<Output = T> + Copy> Div<T> for Point<T> {
    type Output = Self;
    fn div(self, rhs: T) -> Self {
        Self {
            x: self.x / rhs,
            y: self.y / rhs,
        }
    }
}

impl<T: DivAssign<T> + Copy> DivAssign<T> for Point<T> {
    fn div_assign(&mut self, rhs: T) {
        self.x /= rhs;
        self.y /= rhs;
    }
}

impl<X, Y> From<(X, Y)> for Point<f32>
where
    X: ToF32,
    Y: ToF32,
{
    fn from(tup: (X, Y)) -> Self {
        Self {
            x: tup.0.to_f32(),
            y: tup.1.to_f32(),
        }
    }
}

impl<T: Default> From<()> for Point<T> {
    fn from(_value: ()) -> Self {
        Self::default()
    }
}

impl<T: ToF32> Display for Point<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "x: {:.2}, y: {:.2}", self.x.to_f32(), self.y.to_f32())
    }
}
