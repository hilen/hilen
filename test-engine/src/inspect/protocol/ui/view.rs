use refs::Own;
use serde::{Deserialize, Serialize};

use crate::{
    gm::{color::Color, flat::Rect},
    ui::Placer,
};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ViewRepr {
    pub label:    String,
    pub id:       String,
    pub frame:    Rect,
    pub color:    Color,
    pub text:     Option<String>,
    pub hidden:   bool,
    pub placer:   Placer,
    pub subviews: Vec<Own<ViewRepr>>,
}

impl Default for ViewRepr {
    fn default() -> Self {
        Self {
            label:    String::default(),
            id:       String::default(),
            frame:    Rect::default(),
            color:    Color::default(),
            text:     None,
            hidden:   false,
            placer:   Placer::empty(),
            subviews: vec![],
        }
    }
}
