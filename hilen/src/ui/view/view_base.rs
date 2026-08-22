#![allow(clippy::struct_excessive_bools)]

use educe::Educe;

use crate::{
    deps::{
        refs::{Own, Weak},
        vents::{Event, OnceEvent},
    },
    gm::{
        color::Color,
        flat::{CornerRadii, Rect},
    },
    ui::{DynamicColor, Gradient, NavigationView, Shadow, Touch, UIEvent, View, WeakView, layout::Placer},
};

#[derive(Educe)]
#[educe(Default, Debug)]
pub struct ViewBase {
    pub(crate) color: Color,

    #[educe(Debug(ignore))]
    pub(crate) dynamic_color: Option<DynamicColor>,

    #[educe(Debug(ignore))]
    pub(crate) gradient: Option<Gradient>,

    #[educe(Debug(ignore))]
    pub(crate) corner_radii:         CornerRadii,
    #[educe(Debug(ignore))]
    pub(crate) shadow:               Option<Shadow>,
    #[educe(Debug(ignore))]
    pub(crate) border_color:         Color,
    #[educe(Debug(ignore))]
    pub(crate) dynamic_border_color: Option<DynamicColor>,
    #[educe(Debug(ignore))]
    pub(crate) border_width:         f32,

    #[allow(clippy::pub_underscore_fields)]
    pub __content_offset: f32,

    pub(crate) is_hidden: bool,

    #[educe(Default = crate::ui::UIManager::ROOT_VIEW_Z_OFFSET)]
    pub(crate) z_position: f32,

    /// Set through `set_z_position`. Blocks the automatic z assignment
    /// when the view is added to a superview.
    #[educe(Debug(ignore))]
    pub(crate) z_position_custom: bool,

    pub(crate) frame:     Rect,
    #[allow(clippy::pub_underscore_fields)]
    pub __absolute_frame: Rect,

    #[educe(Debug(ignore))]
    pub(crate) superview: WeakView,

    #[educe(Debug(ignore))]
    pub(crate) subviews: Vec<Own<dyn View>>,

    #[educe(Debug(ignore))]
    #[allow(clippy::pub_underscore_fields)]
    pub __touch_id: usize,

    #[educe(Debug(ignore))]
    pub(crate) is_selected: bool,

    #[educe(Debug(ignore))]
    pub(crate) is_hovered: bool,

    #[educe(Debug(ignore))]
    pub(crate) is_system: bool,

    #[educe(Debug(ignore))]
    pub(crate) navigation_view: Weak<NavigationView>,

    pub view_label: String,

    #[educe(Debug(ignore))]
    #[educe(Default = Placer::empty())]
    pub(crate) placer: Placer,

    #[educe(Debug(ignore))]
    pub events: ViewEvents,

    #[educe(Debug(ignore))]
    pub dont_hide_off_screen: bool,

    #[educe(Debug(ignore))]
    pub(crate) trigger_pos_changed:  bool,
    #[educe(Debug(ignore))]
    pub(crate) trigger_size_changed: bool,

    #[educe(Debug(ignore))]
    pub(crate) position_changed: Event,
    #[educe(Debug(ignore))]
    pub(crate) size_changed:     Event,

    pub(crate) ignore_global_style: bool,

    pub tag: usize,
}

impl ViewBase {
    pub(crate) fn __subviews(&self) -> &[Own<dyn View>] {
        &self.subviews
    }
}

#[derive(Default)]
pub struct ViewEvents {
    pub touch: ViewTouchEvents,
    pub setup: OnceEvent,
}

#[derive(Default)]
pub struct ViewTouchEvents {
    pub all:       Event<Touch>,
    pub began:     Event<Touch>,
    pub moved:     Event<Touch>,
    pub up_inside: UIEvent<Touch>,
    /// Fires true on hover enter and false on exit. Only the topmost
    /// hover enabled view under the cursor is hovered. Desktop and the
    /// browser, since a touch screen has no pointer.
    pub hovered:   UIEvent<bool>,
}
