use plat::Platform;
use ui_proc::view;

use crate::{
    deps::refs::{Own, Weak},
    gm::{
        color::CLEAR,
        flat::{Point, Rect, Size},
    },
    ui::{
        ImageMode, ImageView, SwipeBack, UIColor, UIManager, View, ViewData, ViewFrame, ViewSubviews,
        WeakView, view::Setup,
    },
    window::image::{NoImage, ToImage},
};

#[view]
pub struct RootView {
    inner_pos: Point,
    outer_pos: Point,

    inner_size: Size,
    outer_size: Size,

    test_canvas: Option<Size>,

    /// How far the screen keyboard moved the app views up, see
    /// `ScreenKeyboard`.
    keyboard_shift: f32,

    /// Where both layers sit when no swipe moves them.
    layer_origin: Point,

    background: Weak<ImageView>,
    /// The app views.
    screen:     Weak<ScreenLayer>,
    /// The screen a swipe back goes to, behind the app views while the
    /// swipe runs, see `SwipeBack`. Empty at any other time.
    under:      Weak<ScreenLayer>,
}

/// 1 of the 2 layers of the root that hold a screen.
#[view]
pub(crate) struct ScreenLayer {
    /// On while the layer holds the screen under a swipe. It is drawn
    /// first and is 1 step farther away, as a flat layer the app views
    /// drawn after it cover all of it. A later sibling draws behind the
    /// subviews of an earlier one otherwise. A flat layer has to clip.
    covered: bool,
}

impl Setup for ScreenLayer {
    fn clips_to_bounds(&self) -> bool {
        self.covered
    }

    fn setup(self: Weak<Self>) {}
}

impl ScreenLayer {
    fn set_covered(mut self: Weak<Self>, covered: bool) {
        self.covered = covered;
        self.__base_view().flat_depth = covered;
    }
}

impl RootView {
    pub(crate) fn add_subview_to_root(&mut self, view: Own<dyn View>) -> WeakView {
        self.screen.add_subview(view)
    }

    pub(crate) fn setup_root(&mut self) {
        let image = ImageView::new();
        self.background = self.__add_subview_internal(image, true).downcast_view::<ImageView>().unwrap();
        self.background.place().back();

        // Added before the app views, so it is drawn before them. It takes
        // the depth of the background, 1 sibling step farther than the app
        // views, which keep the depth they had before this layer existed.
        let background_z = self.background.z_position();
        let mut under = ScreenLayer::new();
        under.set_z_position(background_z);
        self.under = self.__add_subview_internal(under, true).downcast_view::<ScreenLayer>().unwrap();

        let mut screen = ScreenLayer::new();
        screen.set_z_position(background_z - UIManager::additional_z_offset());
        self.screen = self
            .__add_subview_internal(screen, true)
            .downcast_view::<ScreenLayer>()
            .unwrap();
    }

    pub(crate) fn clear_root(&mut self) {
        SwipeBack::reset();
        self.screen.remove_all_subviews();
        self.under.remove_all_subviews();
        self.under.set_covered(false);
        self.place_layers(0.0, 0.0);
    }

    /// The layer of the app views, which a swipe back moves.
    pub(crate) fn screen_layer(&self) -> WeakView {
        self.screen.weak_view()
    }

    /// Puts the screen a swipe back goes to behind the app views.
    pub(crate) fn add_under(&mut self, view: Own<dyn View>) -> WeakView {
        self.under.set_covered(true);
        self.under.add_subview(view)
    }

    /// Ends a swipe that went back: the screen in the layer under becomes
    /// the app views. The 2 layers trade places, the views in them stay,
    /// so no screen runs its setup again.
    pub(crate) fn raise_under(mut self: Weak<Self>) {
        let (front, back) = (self.screen, self.under);
        let (front_z, back_z) = (front.z_position(), back.z_position());
        back.set_covered(false);
        back.bump_z_position(back_z - front_z);
        back.__base_view().z_position = front_z;
        front.bump_z_position(front_z - back_z);
        front.__base_view().z_position = back_z;

        // The subviews are drawn in their order, the app views last.
        let (front_raw, back_raw) = (front.weak_view().raw(), back.weak_view().raw());
        let subviews = self.subviews_mut();
        let front_index = subviews.iter().position(|view| view.raw() == front_raw);
        let back_index = subviews.iter().position(|view| view.raw() == back_raw);
        if let (Some(front_index), Some(back_index)) = (front_index, back_index) {
            subviews.swap(front_index, back_index);
        }

        front.remove_all_subviews();
        self.screen = back;
        self.under = front;
        self.place_layers(0.0, 0.0);
    }

    /// Drops the screen a swipe that went back to its start had built.
    pub(crate) fn clear_under(self: Weak<Self>) {
        self.under.remove_all_subviews();
        self.under.set_covered(false);
        self.place_layers(0.0, 0.0);
    }

    /// Moves the app views `shift` points right of their place and the
    /// layer under `under_shift` points.
    pub(crate) fn place_layers(&self, shift: f32, under_shift: f32) {
        let origin = self.layer_origin;
        self.screen.set_position((origin.x + shift, origin.y));
        self.under.set_position((origin.x + under_shift, origin.y));
    }

    /// A test that fails part way through never reaches the line that puts the
    /// root background back. Every later test would then probe those leftovers.
    pub(crate) fn reset_background(&mut self) {
        self.background.set_color(CLEAR);
        self.background.set_image(NoImage);
    }

    pub fn set_color(self: Weak<Self>, color: impl Into<UIColor>) -> Weak<Self> {
        self.background.set_color(color.into());
        self
    }

    pub fn set_image(mut self: Weak<Self>, image: impl ToImage) -> Weak<Self> {
        self.background.mode = ImageMode::AspectFill;
        self.background.set_image(image);
        self
    }

    /// UI tests probe fixed screen pixels, so they need a fixed rectangle to
    /// draw in. A device screen cannot be resized to match, so pin the root to
    /// the canvas instead. The rest of the screen just shows the clear color.
    /// Everything that lays out against the root, such as a modal, then lands
    /// where the probes expect it on any screen.
    pub(crate) fn set_test_canvas(mut self: Weak<Self>, canvas: Size) {
        self.test_canvas = canvas.into();
        self.rescale_root(crate::ui::UIManager::scale());
    }

    /// Unpin the root and let it fill the screen again. A run that leaves the
    /// canvas pinned leaves the app boxed into the test's rectangle.
    pub(crate) fn clear_test_canvas(mut self: Weak<Self>) {
        self.test_canvas = None;
        self.rescale_root(crate::ui::UIManager::scale());
    }

    pub(crate) fn resize_root(
        mut self: Weak<Self>,
        inner_pos: Point,
        outer_pos: Point,
        inner_size: Size,
        outer_size: Size,
        scale: f32,
    ) {
        self.inner_pos = inner_pos;
        self.outer_pos = outer_pos;
        self.inner_size = inner_size;
        self.outer_size = outer_size;

        // The canvas is a count of screen pixels, while views lay out in
        // points, so the scale has to be divided back out.
        if let Some(canvas) = self.test_canvas {
            let width = canvas.width * (1.0 / scale);
            let height = canvas.height * (1.0 / scale);

            self.set_size(width, height);
            self.screen.set_size(width, height);
            self.under.set_size(width, height);
            self.layer_origin = Point::new(0.0, -self.keyboard_shift);
            self.place_layers(0.0, 0.0);

            return;
        }

        let render_size = if Platform::DESKTOP {
            self.inner_size
        } else {
            self.outer_size
        };

        self.set_size(
            render_size.width * (1.0 / scale),
            render_size.height * (1.0 / scale),
        );

        // A TV may cut the edges of its picture, and tvOS reports the part
        // that is always seen as the safe area. An app there still draws to
        // the edges, a video and a background, and keeps only its controls
        // inside, see `safe_area`.
        let (size, position) = if Platform::TVOS {
            (render_size, Point::default())
        } else if Platform::IOS {
            (inner_size, inner_pos * (1.0 / scale))
        } else {
            (inner_size, Point::default())
        };

        self.screen.set_size(size.width * (1.0 / scale), size.height * (1.0 / scale));
        self.under.set_size(size.width * (1.0 / scale), size.height * (1.0 / scale));
        self.layer_origin = Point::new(position.x, position.y - self.keyboard_shift);
        self.place_layers(0.0, 0.0);
    }

    /// The part of the app views that no screen cuts and no system bar
    /// covers, in the points of the app views. On a TV it is smaller than
    /// the app views. Everywhere else the app views are that part already,
    /// so it is all of them.
    pub(crate) fn safe_area(&self) -> Rect {
        if Platform::TVOS && self.test_canvas.is_none() {
            let scale = 1.0 / crate::ui::UIManager::scale();
            return (
                self.inner_pos.x * scale,
                self.inner_pos.y * scale,
                self.inner_size.width * scale,
                self.inner_size.height * scale,
            )
                .into();
        }
        let size = self.screen.size();
        (0.0, 0.0, size.width, size.height).into()
    }

    /// Where the app views start on the screen. A view added to them is
    /// placed against this point, not against the screen.
    pub(crate) fn app_views_origin(&self) -> Point {
        self.screen.absolute_frame().origin
    }

    /// The background stays, only the app views move.
    pub(crate) fn set_keyboard_shift(mut self: Weak<Self>, shift: f32) {
        self.keyboard_shift = shift;
        self.rescale_root(crate::ui::UIManager::scale());
    }

    pub(crate) fn rescale_root(self: Weak<Self>, scale: f32) {
        self.resize_root(
            self.inner_pos,
            self.outer_pos,
            self.inner_size,
            self.outer_size,
            scale,
        );
    }
}
