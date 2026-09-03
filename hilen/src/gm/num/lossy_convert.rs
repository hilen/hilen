#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_precision_loss)]

use crate::gm::flat::Rect;

pub trait LossyConvert<To> {
    fn lossy_convert(self) -> To;
}

impl LossyConvert<i32> for f32 {
    fn lossy_convert(self) -> i32 {
        assert!(!self.is_nan(), "Lossy convert from Nan f32");
        assert!(self <= i32::MAX as f32, "Lossy convert overflow");
        assert!(self >= i32::MIN as f32, "Lossy convert overflow");
        self as i32
    }
}

impl LossyConvert<u32> for f32 {
    fn lossy_convert(self) -> u32 {
        assert!(!self.is_nan(), "Lossy convert from Nan f32");
        assert!(self >= 0.0, "Lossy convert sign loss");
        assert!(self <= u32::MAX as f32, "Lossy convert overflow");
        self as u32
    }
}

impl LossyConvert<u64> for f32 {
    fn lossy_convert(self) -> u64 {
        assert!(!self.is_nan(), "Lossy convert from Nan f32");
        assert!(self >= 0.0, "Lossy convert sign loss");
        assert!(self <= u64::MAX as f32, "Lossy convert overflow");
        self as u64
    }
}

impl LossyConvert<i64> for f32 {
    fn lossy_convert(self) -> i64 {
        assert!(!self.is_nan(), "Lossy convert from Nan f32");
        assert!(self <= i64::MAX as f32, "Lossy convert overflow");
        assert!(self > i64::MIN as f32, "Lossy convert underflow");
        self as i64
    }
}

impl LossyConvert<usize> for f32 {
    fn lossy_convert(self) -> usize {
        assert!(!self.is_nan(), "Lossy convert from Nan f32");
        assert!(self >= 0.0, "Lossy convert sign loss");
        assert!(self <= usize::MAX as f32, "Lossy convert overflow");
        self as usize
    }
}

impl LossyConvert<isize> for f32 {
    fn lossy_convert(self) -> isize {
        assert!(!self.is_nan(), "Lossy convert from Nan f32");
        assert!(self <= isize::MAX as f32, "Lossy convert overflow");
        assert!(self >= isize::MIN as f32, "Lossy convert underflow");
        self as isize
    }
}

impl LossyConvert<u8> for f32 {
    fn lossy_convert(self) -> u8 {
        assert!(!self.is_nan(), "Lossy convert from Nan f32");
        assert!(self >= 0.0, "Lossy convert sign loss");
        assert!(self <= f32::from(u8::MAX), "Lossy convert overflow");
        assert!(self >= f32::from(u8::MIN), "Lossy convert underflow");
        self as u8
    }
}

impl LossyConvert<f32> for f64 {
    fn lossy_convert(self) -> f32 {
        assert!(!self.is_nan(), "Lossy convert from Nan f64");
        assert!(self <= f64::from(f32::MAX), "Lossy convert overflow");
        assert!(self >= f64::from(f32::MIN), "Lossy convert underflow");
        self as f32
    }
}

impl LossyConvert<u32> for f64 {
    fn lossy_convert(self) -> u32 {
        assert!(!self.is_nan(), "Lossy convert from Nan f64");
        assert!(self >= 0.0, "Lossy convert sign loss");
        assert!(self <= f64::from(u32::MAX), "Lossy convert overflow");
        assert!(self >= f64::from(u32::MIN), "Lossy convert underflow");
        self as u32
    }
}

impl LossyConvert<u64> for f64 {
    fn lossy_convert(self) -> u64 {
        assert!(!self.is_nan(), "Lossy convert from Nan f64");
        assert!(self >= 0.0, "Lossy convert sign loss");
        assert!(self <= u64::MAX as f64, "Lossy convert overflow");
        self as u64
    }
}

impl LossyConvert<u8> for f64 {
    fn lossy_convert(self) -> u8 {
        assert!(!self.is_nan(), "Lossy convert from Nan f64");
        assert!(self >= 0.0, "Lossy convert sign loss");
        assert!(self <= f64::from(u8::MAX), "Lossy convert overflow");
        assert!(self >= f64::from(u8::MIN), "Lossy convert underflow");
        self as u8
    }
}

// TODO: add some asserts below
impl LossyConvert<f32> for usize {
    fn lossy_convert(self) -> f32 {
        self as f32
    }
}

impl LossyConvert<f32> for u64 {
    fn lossy_convert(self) -> f32 {
        self as f32
    }
}

impl LossyConvert<f32> for i64 {
    fn lossy_convert(self) -> f32 {
        self as f32
    }
}

impl LossyConvert<f64> for i64 {
    fn lossy_convert(self) -> f64 {
        self as f64
    }
}

impl LossyConvert<f32> for u32 {
    fn lossy_convert(self) -> f32 {
        self as f32
    }
}

impl LossyConvert<f32> for i32 {
    fn lossy_convert(self) -> f32 {
        self as f32
    }
}

impl<T: Copy + LossyConvert<U>, U> LossyConvert<Rect<U>> for Rect<T> {
    fn lossy_convert(self) -> Rect<U> {
        Rect::new(
            self.x().lossy_convert(),
            self.y().lossy_convert(),
            self.width().lossy_convert(),
            self.height().lossy_convert(),
        )
    }
}

impl LossyConvert<i64> for f64 {
    fn lossy_convert(self) -> i64 {
        assert!(!self.is_nan(), "Lossy convert from Nan f64");
        assert!(self <= i64::MAX as f64, "Lossy convert overflow");
        assert!(self >= i64::MIN as f64, "Lossy convert underflow");
        self as i64
    }
}

impl LossyConvert<usize> for f64 {
    fn lossy_convert(self) -> usize {
        assert!(!self.is_nan(), "Lossy convert from Nan f64");
        assert!(self >= 0.0, "Lossy convert sign loss");
        assert!(self <= usize::MAX as f64, "Lossy convert overflow");
        self as usize
    }
}
