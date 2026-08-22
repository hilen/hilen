use std::cmp::max;

pub(crate) mod checked_convert;
pub(crate) mod into_f32;
pub(crate) mod lossy_convert;

pub trait Abs {
    fn abs(self) -> Self;
}

impl Abs for f32 {
    fn abs(self) -> Self {
        self.abs()
    }
}

pub trait Zero {
    fn zero() -> Self;
}

macro_rules! impl_zero {
    ($v:expr, $($t:ty),*) => {$(
        impl Zero for $t { fn zero() -> Self { $v } }
    )*};
}

impl_zero!(0, i8, u8, i32, u32, i64, u64, usize);
impl_zero!(0.0, f32, f64);

pub trait IsZero: Zero + Copy {
    fn is_zero(self) -> bool;
}

impl<T: Zero + PartialEq + Copy> IsZero for T {
    fn is_zero(self) -> bool {
        self == Self::zero()
    }
}

pub trait One {
    fn one() -> Self;
}

macro_rules! impl_one {
    ($v:expr, $($t:ty),*) => {$(
        impl One for $t { fn one() -> Self { $v } }
    )*};
}

impl_one!(1, i8, u8, i16, u16, i32, u32, i64, u64, isize, usize);
impl_one!(1.0, f32, f64);

pub trait CheckedSub: Sized {
    fn sub_and_check(&self, other: &Self, min: &Self) -> Option<Self>;
}

impl CheckedSub for u8 {
    fn sub_and_check(&self, other: &Self, min: &Self) -> Option<Self> {
        self.checked_sub(*other).map(|a| max(a, *min))
    }
}

impl CheckedSub for i32 {
    fn sub_and_check(&self, other: &Self, min: &Self) -> Option<Self> {
        self.checked_sub(*other).map(|a| max(a, *min))
    }
}

impl CheckedSub for u32 {
    fn sub_and_check(&self, other: &Self, min: &Self) -> Option<Self> {
        self.checked_sub(*other).map(|a| max(a, *min))
    }
}

impl CheckedSub for i64 {
    fn sub_and_check(&self, other: &Self, min: &Self) -> Option<Self> {
        self.checked_sub(*other).map(|a| max(a, *min))
    }
}

impl CheckedSub for u64 {
    fn sub_and_check(&self, other: &Self, min: &Self) -> Option<Self> {
        self.checked_sub(*other).map(|a| max(a, *min))
    }
}

impl CheckedSub for usize {
    fn sub_and_check(&self, other: &Self, min: &Self) -> Option<Self> {
        self.checked_sub(*other).map(|a| max(a, *min))
    }
}

impl CheckedSub for f32 {
    fn sub_and_check(&self, other: &Self, min: &Self) -> Option<Self> {
        let res = self - other;
        if res < *min { *min } else { res }.into()
    }
}

impl CheckedSub for f64 {
    fn sub_and_check(&self, other: &Self, min: &Self) -> Option<Self> {
        let res = self - other;
        if res < *min { *min } else { res }.into()
    }
}

pub trait MyAdd {
    fn my_add(&self, other: &Self) -> Self;
}

impl MyAdd for f32 {
    fn my_add(&self, other: &Self) -> Self {
        self + other
    }
}

impl MyAdd for f64 {
    fn my_add(&self, other: &Self) -> Self {
        self + other
    }
}

impl MyAdd for u8 {
    fn my_add(&self, other: &Self) -> Self {
        self + other
    }
}

impl MyAdd for i64 {
    fn my_add(&self, other: &Self) -> Self {
        self + other
    }
}

impl MyAdd for u64 {
    fn my_add(&self, other: &Self) -> Self {
        self + other
    }
}

impl MyAdd for i32 {
    fn my_add(&self, other: &Self) -> Self {
        self + other
    }
}

impl MyAdd for u32 {
    fn my_add(&self, other: &Self) -> Self {
        self + other
    }
}

impl MyAdd for usize {
    fn my_add(&self, other: &Self) -> Self {
        self + other
    }
}

pub trait Min {
    fn min() -> Self;
}

macro_rules! impl_min {
    ($($t:ty),*) => {$(
        impl Min for $t { fn min() -> Self { Self::MIN } }
    )*};
}

impl_min!(i8, u8, i32, u32, i64, u64, usize, f32, f64);
