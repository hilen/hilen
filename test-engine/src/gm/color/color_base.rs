use std::{
    fmt::Display,
    ops::{Add, Sub},
};

use arbitrary::Arbitrary;
use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};

use crate::gm::num::{Abs, Zero};

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize, Arbitrary)]
pub struct Color<T = f32> {
    pub r: T,
    pub g: T,
    pub b: T,
    pub a: T,
}

unsafe impl<T: Zeroable> Zeroable for Color<T> {}
unsafe impl<T: Pod> Pod for Color<T> {}

impl<T> Color<T> {
    pub const fn rgba(r: T, g: T, b: T, a: T) -> Self {
        Self { r, g, b, a }
    }

    pub(crate) fn bgra_to_rgba(self) -> Self {
        Self {
            r: self.b,
            g: self.g,
            b: self.r,
            a: self.a,
        }
    }
}

impl<T: Copy> Color<T> {
    pub(crate) const fn as_slice(&self) -> [T; 4] {
        [self.r, self.g, self.b, self.a]
    }

    pub const fn with_r(&self, r: T) -> Self {
        Self {
            r,
            g: self.g,
            b: self.b,
            a: self.a,
        }
    }

    pub const fn with_g(&self, g: T) -> Self {
        Self {
            r: self.r,
            g,
            b: self.b,
            a: self.a,
        }
    }

    pub const fn with_b(&self, b: T) -> Self {
        Self {
            r: self.r,
            g: self.g,
            b,
            a: self.a,
        }
    }

    pub const fn with_alpha(&self, alpha: T) -> Self {
        Self::rgba(self.r, self.g, self.b, alpha)
    }
}

impl<T: Copy + Abs + Sub<Output = T> + Add<Output = T>> Color<T> {
    pub fn diff(&self, other: Color<T>) -> T {
        (self.r - other.r).abs()
            + (self.g - other.g).abs()
            + (self.b - other.b).abs()
            + (self.a - other.a).abs()
    }
}

impl<T: Sub<Output = T> + Copy> Color<T> {
    pub fn lower_by(self, val: T) -> Self {
        Self {
            r: self.r - val,
            g: self.g - val,
            b: self.b - val,
            a: self.a,
        }
    }
}

impl<T: Add<Output = T> + Copy> Color<T> {
    pub fn increase_by(self, val: T) -> Self {
        Self {
            r: self.r + val,
            g: self.g + val,
            b: self.b + val,
            a: self.a,
        }
    }
}

impl<T: Display> Display for Color<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "r: {}, g: {}, b: {}, a: {}", self.r, self.g, self.b, self.a)
    }
}

impl<T: Zero> Default for Color<T> {
    fn default() -> Self {
        Self::rgba(T::zero(), T::zero(), T::zero(), T::zero())
    }
}

impl<T: Default + PartialEq<T>> Color<T> {
    pub(crate) fn is_default(&self) -> bool {
        let def = T::default();
        self.r == def && self.g == def && self.b == def && self.a == def
    }
}

impl<T: Eq> Eq for Color<T> {}
