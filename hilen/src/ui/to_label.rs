use std::{net::IpAddr, path::PathBuf};

use chrono::NaiveDateTime;
use rust_decimal::Decimal;

use crate::gm::flat::Point;

pub trait ToLabel {
    fn to_label(&self) -> String;
}

impl ToLabel for &str {
    fn to_label(&self) -> String {
        (*self).to_string()
    }
}

impl ToLabel for &char {
    fn to_label(&self) -> String {
        self.to_string()
    }
}

impl ToLabel for String {
    fn to_label(&self) -> String {
        self.clone()
    }
}

impl ToLabel for &String {
    fn to_label(&self) -> String {
        (*self).clone()
    }
}

impl ToLabel for PathBuf {
    fn to_label(&self) -> String {
        (*self).to_string_lossy().to_string()
    }
}

impl ToLabel for &PathBuf {
    fn to_label(&self) -> String {
        (*self).to_string_lossy().to_string()
    }
}

impl ToLabel for bool {
    fn to_label(&self) -> String {
        self.to_string()
    }
}

impl ToLabel for u8 {
    fn to_label(&self) -> String {
        self.to_string()
    }
}

impl ToLabel for i8 {
    fn to_label(&self) -> String {
        self.to_string()
    }
}

impl ToLabel for i16 {
    fn to_label(&self) -> String {
        self.to_string()
    }
}

impl ToLabel for u16 {
    fn to_label(&self) -> String {
        self.to_string()
    }
}

impl ToLabel for u32 {
    fn to_label(&self) -> String {
        self.to_string()
    }
}

impl ToLabel for i32 {
    fn to_label(&self) -> String {
        self.to_string()
    }
}

impl ToLabel for i64 {
    fn to_label(&self) -> String {
        self.to_string()
    }
}

impl ToLabel for u64 {
    fn to_label(&self) -> String {
        self.to_string()
    }
}

impl ToLabel for usize {
    fn to_label(&self) -> String {
        self.to_string()
    }
}

impl ToLabel for &usize {
    fn to_label(&self) -> String {
        self.to_string()
    }
}

impl ToLabel for isize {
    fn to_label(&self) -> String {
        self.to_string()
    }
}

impl ToLabel for f32 {
    fn to_label(&self) -> String {
        format!("{self:.2}")
    }
}

impl ToLabel for f64 {
    fn to_label(&self) -> String {
        format!("{self:.2}")
    }
}

impl ToLabel for Decimal {
    fn to_label(&self) -> String {
        format!("{self:.2}")
    }
}

impl ToLabel for Point {
    fn to_label(&self) -> String {
        format!("x: {} y: {}", self.x.to_label(), self.y.to_label())
    }
}

impl ToLabel for IpAddr {
    fn to_label(&self) -> String {
        format!("{self}")
    }
}

// TODO: think about timezones
impl ToLabel for NaiveDateTime {
    fn to_label(&self) -> String {
        self.format("%Y-%m-%d %H:%M:%S").to_string()
    }
}
