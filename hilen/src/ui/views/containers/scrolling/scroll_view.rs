use std::ops::{DerefMut, Neg};

use crate::{
    self as hilen,
    deps::{
        refs::{Own, Weak, weak_from_ref},
        vents::Event,
    },
    gm::{
        ToF32,
        color::Color,
        flat::{Point, Size},
    },
    ui::{
        Container, DynamicColor, Input, NO_TOUCH_ID, Scrollable, Setup, Touch, TouchStack, UIAnimation,
        UIEvent, UIManager, View, ViewCallbacks, ViewData, ViewFrame, ViewSubviews, view,
        views::containers::scrolling::ScrollContent,
    },
};

const BAR_WIDTH: f32 = 4.0;
const BAR_INSET: f32 = 2.0;
const BAR_MIN_LENGTH: f32 = 20.0;
// Translucent black over light content, translucent white over dark,
// a black thumb disappears on a dark theme.
const BAR_COLOR: DynamicColor =
    DynamicColor::new(Color::rgba(0.0, 0.0, 0.0, 0.35), Color::rgba(1.0, 1.0, 1.0, 0.35));

/// A captured touch becomes a drag only after moving this far. Until then
/// taps on views inside the scroll work; after, the drag claims the touch.
const DRAG_SLOP: f32 = 10.0;

/// Which content dimensions the app pinned through the `set_content`
/// calls. Automatic content sizing skips a pinned axis.
#[derive(Default)]
struct ManualContent {
    width:  bool,
    height: bool,
}

#[view]
pub struct ScrollView {
    inertia:            f32,
    inertia_x:          f32,
    began_touch:        Point,
    previous_touch:     Point,
    dragging:           bool,
    /// The drag in flight moves the content sideways. A drag keeps the
    /// axis it started on, so a finger going down a list does not also
    /// shake it left and right.
    drag_sideways:      bool,
    manual_content:     ManualContent,
    drag_disabled:      bool,
    /// The sideways offset: 0 at the left edge, negative right of it.
    offset_x:           f32,
    /// Set by a host that moves its own views sideways, the table. The
    /// offset then ranges over this width and the content stays put.
    sideways_width:     Option<f32>,
    pub on_scroll:      Event<f32>,
    /// The new sideways offset after it changed.
    pub on_scroll_x:    Event<f32>,
    pub bottom_reached: UIEvent,

    #[init]
    pub(crate) content: ScrollContent,

    /// The scroll indicator on the right edge, its length the visible
    /// share of the content and its position the offset. Shown only
    /// when the content is taller than the view.
    bar:   Container,
    /// The same indicator on the bottom edge, shown only when the
    /// content is wider than the view.
    bar_x: Container,
}

impl ScrollView {
    pub fn remove_all_subviews(&mut self) {
        self.content.remove_all_subviews();
    }

    // Content offset must be negative
    fn max_offset(&self) -> f32 {
        (self.content.content_size.height - self.height()).neg().min(0.0)
    }

    /// A drag no longer scrolls, the wheel and `set_content_offset` still
    /// do. For a host that needs the drag itself, like a text field
    /// selecting text.
    pub fn disable_drag(&mut self) -> &mut Self {
        self.drag_disabled = true;
        self
    }

    pub fn set_content_offset(&mut self, offset: impl ToF32) -> &mut Self {
        self.content.__base_view().__content_offset = offset.to_f32();

        if self.content.__base_view().__content_offset < self.max_offset() {
            self.content.__base_view().__content_offset = self.max_offset();
        }

        self
    }

    pub fn set_content_size(&mut self, size: impl Into<Size>) -> &mut Self {
        self.manual_content.width = true;
        self.manual_content.height = true;
        self.content.content_size = size.into();
        self.clamp_offset_x();
        self
    }

    pub fn set_content_width(&mut self, width: impl ToF32) -> &mut Self {
        self.manual_content.width = true;
        self.content.content_size.width = width.to_f32();
        self.clamp_offset_x();
        self
    }

    /// Sets the sideways scroll position: 0 is the left edge, negative
    /// values scroll right. Clamped to the scrollable range.
    pub fn set_content_offset_x(&mut self, offset: impl ToF32) -> &mut Self {
        self.offset_x = offset.to_f32().clamp(-self.range_x(), 0.0);
        self.apply_offset_x();
        self
    }

    /// The sideways scroll position: 0 at the left edge, negative right
    /// of it.
    pub fn content_offset_x(&self) -> f32 {
        self.offset_x
    }

    pub(crate) fn set_sideways_width(&mut self, width: f32) {
        self.sideways_width = Some(width);
        self.clamp_offset_x();
    }

    pub(crate) fn has_sideways_width(&self) -> bool {
        self.sideways_width.is_some()
    }

    /// How far the content can move sideways, 0 when it fits.
    fn range_x(&self) -> f32 {
        let content = self.sideways_width.unwrap_or(self.content.content_size.width);
        (content - self.width()).max(0.0)
    }

    /// The same test `on_scroll` uses to refuse a vertical scroll.
    fn has_vertical_range(&self) -> bool {
        self.content.content_size.height > self.content.height()
    }

    fn apply_offset_x(&mut self) {
        self.content.__base_view().__content_offset_x = if self.sideways_width.is_some() {
            0.0
        } else {
            self.offset_x
        };
    }

    fn clamp_offset_x(&mut self) {
        let min = -self.range_x();
        if self.offset_x >= min {
            return;
        }
        self.offset_x = min;
        self.apply_offset_x();
        self.on_scroll_x.trigger(min);
    }

    pub fn set_content_height(&mut self, height: impl ToF32) -> &mut Self {
        self.manual_content.height = true;
        self.content.content_size.height = height.to_f32();
        self.clamp_offset();
        self
    }

    fn clamp_offset(&mut self) {
        if self.content.__base_view().__content_offset < self.max_offset() {
            self.content.__base_view().__content_offset = self.max_offset();
        }
    }

    pub fn content_height(&self) -> f32 {
        self.content.content_size.height
    }

    pub fn get_scroll_content_offset(&self) -> f32 {
        self.content.content_offset()
    }
}

impl ViewCallbacks for ScrollView {
    /// Content dimensions the app never set follow the layout on their
    /// own: width tracks the viewport, height tracks the lowest subview
    /// edge. Frames are read from the previous layout pass, so the size
    /// settles a frame after the content does.
    fn update(&mut self) {
        if !self.manual_content.width {
            self.content.content_size.width = self.width();
        }
        self.clamp_offset_x();
        if !self.manual_content.height {
            let bottom = self
                .content
                .subviews()
                .iter()
                .filter(|view| !view.is_hidden())
                .map(|view| view.frame().max_y())
                .fold(0.0, f32::max);
            self.content.content_size.height = bottom;
            self.clamp_offset();
        }

        self.update_bar();
        self.update_bar_x();
    }
}

impl ScrollView {
    fn update_bar(&mut self) {
        let height = self.height();
        let content = self.content.content_size.height;

        if content <= height || height <= 0.0 {
            self.bar.set_hidden(true);
            return;
        }

        let track = height - BAR_INSET * 2.0;
        let length = (track * height / content).max(BAR_MIN_LENGTH).min(track);
        let offset = -self.content.__base_view().__content_offset;
        let y = BAR_INSET + (track - length) * offset / (content - height);

        self.bar.set_hidden(false);
        self.bar.set_frame((self.width() - BAR_WIDTH - BAR_INSET, y, BAR_WIDTH, length));
    }
}

impl ScrollView {
    fn update_bar_x(&mut self) {
        let width = self.width();
        let range = self.range_x();

        if range <= 0.0 || width <= 0.0 {
            self.bar_x.set_hidden(true);
            return;
        }

        // Stops short of the vertical bar, so the two never cross in the
        // corner.
        let corner = if self.bar.is_hidden() {
            0.0
        } else {
            BAR_WIDTH + BAR_INSET
        };
        let track = width - BAR_INSET * 2.0 - corner;
        let length = (track * width / (width + range)).max(BAR_MIN_LENGTH).min(track);
        let x = BAR_INSET + (track - length) * -self.offset_x / range;

        self.bar_x.set_hidden(false);
        self.bar_x
            .set_frame((x, self.height() - BAR_WIDTH - BAR_INSET, length, BAR_WIDTH));
    }
}

impl Setup for ScrollView {
    fn clips_to_bounds(&self) -> bool {
        true
    }

    fn setup(mut self: Weak<Self>) {
        self.content.__base_view().dont_hide_off_screen = true;
        self.content.place().back();

        self.bar.set_color(BAR_COLOR).set_corner_radius(BAR_WIDTH / 2.0);
        self.bar.set_hidden(true);
        // A later sibling draws behind an earlier sibling's children, so
        // the bar has to be pushed in front of everything in the content,
        // pinned sticky table rows and their raise included.
        self.bar.bump_z_position(UIManager::subview_z_offset() * 10.0);

        self.bar_x.set_color(BAR_COLOR).set_corner_radius(BAR_WIDTH / 2.0);
        self.bar_x.set_hidden(true);
        self.bar_x.bump_z_position(UIManager::subview_z_offset() * 10.0);

        self.size_changed().sub(move || {
            self.on_scroll(0.0);
        });

        TouchStack::enable_scroll(self);
    }
}

impl ViewSubviews for ScrollView {
    fn remove_all_subviews(&self) {
        self.content.remove_all_subviews();
    }

    fn add_subview<V: ?Sized + View + 'static>(&self, view: Own<V>) -> Weak<V> {
        self.content.add_subview(view)
    }
}

impl Scrollable for ScrollView {
    fn __process_scroll_touch(&mut self, touch: Touch) -> bool {
        if touch.is_ended() {
            // Only the finger this scroll was following ends its drag. A
            // different finger lifting elsewhere must not clear this scroll's
            // capture, or a second scroll dragged at the same time would stop.
            if touch.id == self.__base_view().__touch_id {
                self.add_inertia_animation();
                self.__base_view().__touch_id = NO_TOUCH_ID;
                self.dragging = false;
            }
            return false;
        }

        // Drag scrolling is a touch gesture. On desktop it is off by
        // default so a mouse drag reaches the views under it, text
        // selection first of all, see `UIManager::set_drag_scrolling`.
        if self.is_hidden_in_tree() || self.drag_disabled || !UIManager::drag_scrolling() {
            return false;
        }

        let mut target_frame = self.content.__base_view().__absolute_frame;
        target_frame.origin.y -= self.content.__base_view().__content_offset;
        target_frame.origin.x -= self.content.__base_view().__content_offset_x;

        // A scroll already dragged by one finger keeps following that finger
        // and ignores a second one, so two fingers never fight over it.
        if touch.is_began()
            && self.__base_view().__touch_id == NO_TOUCH_ID
            && target_frame.contains(touch.position)
        {
            self.__base_view().__touch_id = touch.id;
            self.began_touch = touch.position;
            self.previous_touch = touch.position;
            return true;
        }

        if touch.is_moved() && self.__base_view().__touch_id == touch.id {
            if !self.dragging {
                let moved = touch.position - self.began_touch;
                let sideways = self.range_x() > 0.0 && moved.x.abs() > moved.y.abs();
                let travel = if sideways { moved.x.abs() } else { moved.y.abs() };
                if travel < DRAG_SLOP {
                    return true;
                }
                self.dragging = true;
                self.drag_sideways = sideways;
                if sideways {
                    self.inertia = 0.0;
                } else {
                    self.inertia_x = 0.0;
                }
                self.previous_touch = self.began_touch;
                TouchStack::cancel_touch(touch.id);
                // cancel_touch clears every capture, including this scroll's
                // if it is also a touch listener
                self.__base_view().__touch_id = touch.id;
            }

            if self.drag_sideways {
                let delta = touch.position.x - self.previous_touch.x;
                self.previous_touch = touch.position;

                if delta != 0.0 {
                    self.inertia_x = delta;
                    self.scroll_x(delta);
                }
                return true;
            }

            let delta = -(self.previous_touch.y - touch.position.y);
            self.previous_touch = touch.position;

            if delta == 0.0 {
                return true;
            }

            self.inertia = delta;
            self.on_scroll(delta);
            return true;
        }

        false
    }

    fn __process_wheel_scroll(&mut self, delta: Point) {
        // A plain mouse wheel has only the vertical delta. Over a view
        // that can move only sideways it would do nothing, so it moves
        // that view sideways, with no key held.
        if delta.x == 0.0 && self.range_x() > 0.0 && !self.has_vertical_range() {
            self.scroll_x(delta.y);
            return;
        }

        self.__scroll_by(delta);
    }

    fn __scroll_by(&mut self, delta: Point) {
        // A mouse with no side wheel scrolls sideways with Shift held.
        // macOS turns that into a sideways delta by itself, the other
        // platforms still send it as a vertical one.
        if delta.x == 0.0 && Input::modifiers().shift_key() && self.range_x() > 0.0 {
            self.scroll_x(delta.y);
            return;
        }

        self.on_scroll(delta.y);
        self.scroll_x(delta.x);
    }
}

impl ScrollView {
    /// A finger drags it or the fling after a drag still moves it. The
    /// fling decays by a fixed factor per frame, so it ends at the same
    /// offset on any frame rate, only sooner or later. A test that taps
    /// rows after a drag waits for this before it taps.
    pub fn is_scrolling(&self) -> bool {
        self.dragging || self.inertia != 0.0 || self.inertia_x != 0.0
    }

    fn add_inertia_animation(&self) {
        if self.inertia == 0.0 && self.inertia_x == 0.0 {
            return;
        }

        let mut scroll = weak_from_ref(self);

        let anim = UIAnimation::new(move |_, _| {
            if scroll.inertia_x == 0.0 {
                let inertia = scroll.inertia;
                scroll.on_scroll(inertia);
                scroll.inertia *= 0.97;
            } else {
                let inertia = scroll.inertia_x;
                scroll.scroll_x(inertia);
                scroll.inertia_x *= 0.97;
            }
        })
        .finish_condition(move || {
            if scroll.inertia.abs() > 0.2 || scroll.inertia_x.abs() > 0.2 {
                return false;
            }
            scroll.inertia = 0.0;
            scroll.inertia_x = 0.0;
            true
        });

        self.add_animation(anim);
    }

    fn scroll_x(&mut self, delta: f32) {
        let range = self.range_x();
        if range <= 0.0 {
            return;
        }

        let offset = (self.offset_x + delta).clamp(-range, 0.0);
        if (offset - self.offset_x).abs() < f32::EPSILON {
            return;
        }

        self.offset_x = offset;
        self.apply_offset_x();
        self.on_scroll_x.trigger(offset);
    }

    fn on_scroll(&mut self, scroll: f32) {
        let height = self.content.height();
        let content = self.content.deref_mut();

        if height >= content.content_size.height {
            return;
        }

        *content.content_offset_mut() += scroll;
        let range = content.content_size.height - height;

        if *content.content_offset_mut() <= -range {
            self.bottom_reached.trigger(());
        }

        *content.content_offset_mut() = content.content_offset_mut().clamp(-range, 0.0);

        self.on_scroll.trigger(*content.content_offset_mut());
    }
}
