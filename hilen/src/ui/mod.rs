mod accessibility;
#[cfg(feature = "code-colors")]
mod code_highlighter;
#[cfg(feature = "gpu")]
pub(crate) mod damage;
mod focus;
mod gradient;
mod hover;
mod images;
mod input;
#[cfg(feature = "gpu")]
mod label_drawer;
#[cfg(feature = "gpu")]
mod label_effect;
mod layout;
pub(crate) mod loops;
mod modal_view;
#[cfg(feature = "ui-tests")]
mod navigation_events_test;
#[cfg(feature = "ui-tests")]
mod navigation_shell_swipe_test;
#[cfg(feature = "ui-tests")]
mod navigation_swipe_test;
mod navigation_view;
pub(crate) mod privacy_cover;
#[cfg(feature = "ui-tests")]
mod privacy_cover_test;
mod screen_keyboard;
#[cfg(feature = "gpu")]
mod selection_drawer;
mod shadow;
mod side_swipe;
#[cfg(feature = "ui-tests")]
mod side_swipe_slide_test;
#[cfg(feature = "ui-tests")]
mod side_swipe_test;
mod style;
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
#[cfg(feature = "gpu")]
pub(crate) mod ui_drawer;
mod ui_event;
mod ui_manager;
mod view;
mod views;
pub(crate) mod width_class;
mod with_header;

pub mod mobile;
pub(crate) mod serde;
pub(crate) mod ui_dispatch;
#[cfg(feature = "gpu")]
pub mod ui_test;

pub use ui_proc::*;

#[cfg(feature = "code-colors")]
pub use self::code_highlighter::CodeHighlighter;
#[cfg(feature = "gpu")]
pub use self::ui_drawer::{DrawsItself, UIDrawer};
pub use self::{
    accessibility::{Accessibility, AccessibleNode, AccessibleRole, ViewAccessibility},
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
    text_field_constraint::*,
    text_selection::TextSelection,
    theme::*,
    to_label::*,
    toast::*,
    tooltip::*,
    touch_stack::*,
    ui_event::*,
    ui_manager::*,
    view::*,
    views::*,
    width_class::WidthClass,
    with_header::*,
};
pub(crate) use self::{focus::FocusData, touch_layer::*};
#[cfg(feature = "gpu")]
pub use crate::window::PolygonMode;
pub use crate::{
    gm::{
        color::*,
        flat::{
            CornerRadii, FillRule, LineCap, LineJoin, Paint, Point, PointsPath, Ramp, Rect, Size,
            StrokeStyle, VectorPath, VectorPathBuilder,
        },
    },
    image::{Image, ImageFilter, NoImage, Tinted},
    text::Font,
    window::Screenshot,
};

pub const ALL_VIEWS: &[&str] = &all_views!();
