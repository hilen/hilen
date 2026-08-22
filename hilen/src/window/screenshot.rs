use std::path::Path;

use anyhow::Result;
use image::{ColorType::Rgba8, save_buffer};

use crate::gm::{
    LossyConvert,
    color::{Color, U8Color},
    flat::{Point, Size},
};

#[derive(Debug, Default)]
pub struct Screenshot {
    pub data: Vec<U8Color>,
    pub size: Size<u32>,
}

impl Screenshot {
    pub fn new(data: Vec<U8Color>, size: Size<u32>) -> Self {
        Self { data, size }
    }

    pub fn get_pixel(&self, pos: impl Into<Point>) -> U8Color {
        if self.data.is_empty() {
            return Color::default();
        }

        let pos: Point<usize> = pos.into().lossy_convert();

        let Some(color) = self.data.get(pos.x + pos.y * self.size.width as usize) else {
            return Color::default();
        };

        *color
    }

    pub(crate) fn save(&self, path: &Path) -> Result<()> {
        let mut bytes = Vec::with_capacity(self.data.len() * 4);

        for color in &self.data {
            bytes.extend_from_slice(&[color.r, color.g, color.b, 255]);
        }

        save_buffer(path, &bytes, self.size.width, self.size.height, Rgba8)?;

        Ok(())
    }
}
