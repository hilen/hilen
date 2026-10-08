#![allow(clippy::struct_excessive_bools)]

use educe::Educe;

use crate::{
    deps::{
        refs::{Own, Weak},
        vents::{Event, OnceEvent},
    },
    gm::{
        color::Color,
        flat::{CornerRadii, Point, Rect},
    },
    ui::{
        CursorIcon, DynamicColor, FocusData, Gradient, NavigationView, Pinch, Shadow, TooltipContent, Touch,
        UIEvent, View, WeakView, layout::Placer, view::DoubleTap,
    },
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
    pub __content_offset:   f32,
    #[allow(clippy::pub_underscore_fields)]
    pub __content_offset_x: f32,

    /// Follows the sideways offset of the table it sits in, see
    /// `ViewData::set_moves_sideways`.
    pub(crate) moves_sideways: bool,

    pub(crate) is_hidden: bool,

    /// See `ViewData::set_opacity`.
    #[educe(Default = 1.0)]
    pub(crate) opacity:       f32,
    /// This view's opacity times the opacity of every view above it, set
    /// on each update, so the drawer reads one number per view.
    #[educe(Default = 1.0)]
    pub(crate) tree_opacity:  f32,
    /// See `ViewData::set_group_opacity`.
    pub(crate) group_opacity: bool,
    /// The opacity the picture of this view's group is drawn with, 1 when
    /// the view is no group or has nothing to fade. A group draws its own
    /// parts at full strength, so its `tree_opacity` is 1.
    #[educe(Default = 1.0)]
    pub(crate) group_alpha:   f32,

    #[educe(Default = crate::ui::UIManager::ROOT_VIEW_Z_OFFSET)]
    pub(crate) z_position: f32,

    /// Set through `set_z_position`. Blocks the automatic z assignment
    /// when the view is added to a superview.
    #[educe(Debug(ignore))]
    pub(crate) z_position_custom: bool,

    pub(crate) frame:     Rect,
    #[allow(clippy::pub_underscore_fields)]
    pub __absolute_frame: Rect,

    /// How much bigger this view draws its subviews, 1 by default. Only
    /// a `CanvasView` sets it.
    #[educe(Default = 1.0)]
    pub(crate) content_scale: f32,
    /// Where the zero point of the subviews sits inside this view, in its
    /// own points.
    pub(crate) content_shift: Point,
    /// The scale this view is drawn at, the `content_scale` of every view
    /// above it multiplied, set on each update. The absolute frame is in
    /// screen points, so it is `frame` times this number.
    #[educe(Default = 1.0)]
    pub(crate) tree_scale:    f32,
    /// Everything inside this view counts as one flat layer at the depth
    /// of the view, for drawing and for input. A view drawn after it, a
    /// button over a map, is then in front of all of it. Without this a
    /// later sibling draws behind the subviews of an earlier one. Only
    /// for a view that clips to its bounds.
    pub(crate) flat_depth:    bool,

    #[educe(Debug(ignore))]
    pub(crate) superview: WeakView,

    #[educe(Debug(ignore))]
    pub(crate) subviews: Vec<Own<dyn View>>,

    #[educe(Debug(ignore))]
    #[allow(clippy::pub_underscore_fields)]
    pub __touch_id: usize,

    /// The tap before, to tell a double tap.
    pub(crate) taps: DoubleTap,

    #[educe(Debug(ignore))]
    pub(crate) is_selected: bool,

    /// See `ViewTouch::set_keeps_selection`.
    pub(crate) keeps_selection: bool,

    #[educe(Debug(ignore))]
    pub(crate) is_hovered: bool,

    #[educe(Debug(ignore))]
    pub(crate) is_hover_within: bool,

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

    #[educe(Debug(ignore))]
    pub(crate) tooltip: Option<TooltipContent>,

    #[educe(Debug(ignore))]
    pub(crate) hover_cursor: Option<CursorIcon>,

    pub tag: usize,

    /// The frame is erased inside this view, so a browser shows the page
    /// behind the canvas there. Set by a `VideoView` that plays in a
    /// `<video>` element, read only in a browser.
    pub(crate) page_hole: bool,

    /// What the key focus keeps for this view, see `ui/focus.rs`.
    #[educe(Debug(ignore))]
    pub(crate) focus: FocusData,
}

impl ViewBase {
    pub(crate) fn __subviews(&self) -> &[Own<dyn View>] {
        &self.subviews
    }

    /// The absolute frame in the points this view lays out in. Drawn with
    /// the screen scale times `tree_scale` it lands on the same pixels as
    /// the absolute frame, and a radius, a border and a text size given
    /// in points grow with the view.
    pub(crate) fn draw_frame(&self) -> Rect {
        if self.tree_scale.to_bits() == 1.0_f32.to_bits() {
            return self.__absolute_frame;
        }
        self.__absolute_frame * (1.0 / self.tree_scale)
    }

    /// A point of the screen in the points of this view.
    pub(crate) fn local_point(&self, screen: Point) -> Point {
        (screen - self.__absolute_frame.origin) / self.tree_scale
    }
}

#[derive(Default)]
pub struct ViewEvents {
    pub touch: ViewTouchEvents,
    pub setup: OnceEvent,
}

#[derive(Default)]
pub struct ViewTouchEvents {
    pub all:          Event<Touch>,
    pub began:        Event<Touch>,
    pub moved:        Event<Touch>,
    pub up_inside:    UIEvent<Touch>,
    /// The second of 2 taps that come soon after each other at one place,
    /// a double click with a mouse. Fires after the `up_inside` of that
    /// tap. The first tap of the pair fired its `up_inside` at once, with
    /// no wait. The tap after a pair starts a new pair.
    pub double_tap:   UIEvent<Touch>,
    /// A tap that no second tap followed, for a view where 1 tap and a
    /// double tap do 2 different things, like a video that pauses on a
    /// click and goes fullscreen on a double click. It fires only after
    /// the time a second tap may take, a quarter of a second. A view
    /// with no double tap uses `up_inside`, which has no wait.
    pub single_tap:   UIEvent<Touch>,
    /// Fires true on hover enter and false on exit. Only the topmost
    /// hover enabled view under the cursor is hovered. Desktop and the
    /// browser, since a touch screen has no pointer.
    pub hovered:      UIEvent<bool>,
    /// Fires true when the hovered view becomes this view or any view
    /// inside it, and false when hover leaves the whole subtree, like CSS
    /// `:hover` on a parent. Moving from a row onto its own button fires
    /// nothing on the row. Only a hover enabled view is ever hovered, so
    /// a view that wants its own area to count calls `enable_hover` too.
    pub hover_within: UIEvent<bool>,
    /// A right click on desktop and in the browser, a long press on a
    /// touch screen. The touch position is in the view's own coordinates.
    /// A long press consumes the hold, so its release is not a tap.
    pub secondary:    UIEvent<Touch>,
    /// Every step of a pinch over the view, 2 fingers on a touch screen
    /// or a pinch on a trackpad. Only after `enable_pinch`. The frontmost
    /// such view under the pinch gets it.
    pub pinch:        UIEvent<Pinch>,
}
