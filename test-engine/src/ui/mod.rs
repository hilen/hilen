mod gradient;
mod hover;
mod images;
mod input;
mod layout;
mod modal_view;
mod navigation_view;
mod shadow;
mod style;
mod tests;
mod text_field_constraint;
mod theme;
mod to_label;
mod touch_layer;
mod touch_stack;
pub(crate) mod ui_drawer;
mod ui_event;
mod ui_manager;
mod view;
mod views;
mod with_header;

pub mod mobile;
pub(crate) mod serde;
pub(crate) mod ui_dispatch;
pub mod ui_test;

pub use ui_proc::*;

pub(crate) use self::touch_layer::*;
pub use self::{
    gradient::*, hover::*, images::*, input::*, layout::*, modal_view::*, navigation_view::*, shadow::*,
    style::*, text_field_constraint::*, theme::*, to_label::*, touch_stack::*, ui_drawer::UIDrawer,
    ui_event::*, ui_manager::*, view::*, views::*, with_header::*,
};
pub use crate::{
    gm::{
        color::*,
        flat::{
            CornerRadii, FillRule, LineCap, LineJoin, Point, PointsPath, Rect, Size, StrokeStyle, VectorPath,
            VectorPathBuilder,
        },
    },
    window::{
        Font, PolygonMode, Screenshot,
        image::{Image, NoImage, Tinted},
    },
};

pub const ALL_VIEWS: &[&str] = &all_views!();
