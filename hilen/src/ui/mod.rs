mod code_highlighter;
pub(crate) mod damage;
mod focus;
mod gradient;
mod hover;
mod images;
mod input;
mod label_drawer;
mod label_effect;
mod layout;
pub(crate) mod loops;
mod modal_view;
mod navigation_view;
pub(crate) mod privacy_cover;
#[cfg(feature = "ui-tests")]
mod privacy_cover_test;
mod screen_keyboard;
mod selection_drawer;
mod shadow;
mod side_swipe;
#[cfg(feature = "ui-tests")]
mod side_swipe_slide_test;
#[cfg(feature = "ui-tests")]
mod side_swipe_test;
mod style;
mod swipe_back;
#[cfg(feature = "ui-tests")]
mod swipe_back_test;
#[cfg(feature = "ui-tests")]
mod tests;
mod text_field_constraint;
mod text_selection;
mod theme;
mod to_label;
mod toast;
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
    loops::LoopAnimation,
    modal_view::*,
    navigation_view::*,
    screen_keyboard::{ScreenKeyboard, ScreenKeyboardMove},
    shadow::*,
    side_swipe::{SideSwipe, SideSwipeEnd, SideSwipeEvent},
    style::*,
    swipe_back::SwipeBack,
    text_field_constraint::*,
    text_selection::TextSelection,
    theme::*,
    to_label::*,
    toast::*,
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
