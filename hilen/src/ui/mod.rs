mod code_highlighter;
mod focus;
mod gradient;
mod hover;
mod images;
mod input;
mod label_drawer;
mod label_effect;
mod layout;
mod modal_view;
mod navigation_view;
mod screen_keyboard;
mod selection_drawer;
mod shadow;
mod style;
#[cfg(feature = "ui-tests")]
mod tests;
mod text_field_constraint;
mod text_selection;
mod theme;
mod to_label;
mod tooltip;
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

pub use self::{
    code_highlighter::CodeHighlighter,
    focus::{Focus, FocusDirection, ViewFocus},
    gradient::*,
    hover::*,
    images::*,
    input::*,
    layout::*,
    modal_view::*,
    navigation_view::*,
    screen_keyboard::{ScreenKeyboard, ScreenKeyboardMove},
    shadow::*,
    style::*,
    text_field_constraint::*,
    text_selection::TextSelection,
    theme::*,
    to_label::*,
    tooltip::*,
    touch_stack::*,
    ui_drawer::UIDrawer,
    ui_event::*,
    ui_manager::*,
    view::*,
    views::*,
    with_header::*,
};
pub(crate) use self::{focus::FocusData, touch_layer::*};
pub use crate::{
    gm::{
        color::*,
        flat::{
            CornerRadii, FillRule, LineCap, LineJoin, Paint, Point, PointsPath, Ramp, Rect, Size,
            StrokeStyle, VectorPath, VectorPathBuilder,
        },
    },
    window::{
        Font, PolygonMode, Screenshot,
        image::{Image, ImageFilter, NoImage, Tinted},
    },
};

pub const ALL_VIEWS: &[&str] = &all_views!();
