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
        Container, NO_TOUCH_ID, Scrollable, Setup, Touch, TouchStack, UIAnimation, UIEvent, UIManager, View,
        ViewCallbacks, ViewData, ViewFrame, ViewSubviews, view, views::containers::scrolling::ScrollContent,
    },
};

const BAR_WIDTH: f32 = 4.0;
const BAR_INSET: f32 = 2.0;
const BAR_MIN_LENGTH: f32 = 20.0;
const BAR_COLOR: Color = Color::rgba(0.0, 0.0, 0.0, 0.35);

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
    began_touch:        Point,
    previous_touch:     Point,
    dragging:           bool,
    manual_content:     ManualContent,
    drag_disabled:      bool,
    pub on_scroll:      Event<f32>,
    pub bottom_reached: UIEvent,

    #[init]
    pub(crate) content: ScrollContent,

    /// The scroll indicator on the right edge, its length the visible
    /// share of the content and its position the offset. Shown only
    /// when the content is taller than the view.
    bar: Container,
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
        self
    }

    pub fn set_content_width(&mut self, width: impl ToF32) -> &mut Self {
        self.manual_content.width = true;
        self.content.content_size.width = width.to_f32();
        self
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
        // the bar has to be pushed in front of everything in the content.
        self.bar.bump_z_position(UIManager::subview_z_offset() * 2.0);

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

        if self.is_hidden_in_tree() || self.drag_disabled {
            return false;
        }

        let mut target_frame = self.content.__base_view().__absolute_frame;
        target_frame.origin.y -= self.content.__base_view().__content_offset;

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
                if (touch.position.y - self.began_touch.y).abs() < DRAG_SLOP {
                    return true;
                }
                self.dragging = true;
                self.previous_touch = self.began_touch;
                TouchStack::cancel_touch(touch.id);
                // cancel_touch clears every capture, including this scroll's
                // if it is also a touch listener
                self.__base_view().__touch_id = touch.id;
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
        self.on_scroll(delta.y);
    }
}

impl ScrollView {
    fn add_inertia_animation(&self) {
        if self.inertia == 0.0 {
            return;
        }

        let mut scroll = weak_from_ref(self);

        let anim = UIAnimation::new(move |_, _| {
            let inertia = scroll.inertia;
            scroll.on_scroll(inertia);
            scroll.inertia *= 0.97;
        })
        .finish_condition(move || scroll.inertia.abs() <= 0.2);

        self.add_animation(anim);
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
