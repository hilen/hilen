use plat::Platform;
use ui_proc::view;

use crate::{
    deps::refs::{Own, Weak},
    gm::{
        color::CLEAR,
        flat::{Point, Rect, Size},
    },
    ui::{
        Container, ImageMode, ImageView, UIColor, View, ViewData, ViewFrame, ViewSubviews, WeakView,
        view::Setup,
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

    background: Weak<ImageView>,
    screen:     Weak<Container>,
}

impl RootView {
    pub(crate) fn add_subview_to_root(&mut self, view: Own<dyn View>) -> WeakView {
        self.screen.add_subview(view)
    }

    pub(crate) fn setup_root(&mut self) {
        let image = ImageView::new();
        self.background = self.__add_subview_internal(image, true).downcast_view::<ImageView>().unwrap();
        self.background.place().back();

        let screen = Container::new();
        self.screen = self.__add_subview_internal(screen, true).downcast_view::<Container>().unwrap();
    }

    pub(crate) fn clear_root(&mut self) {
        self.screen.remove_all_subviews();
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
            self.screen.set_position((0, -self.keyboard_shift));

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
        self.screen.set_position((position.x, position.y - self.keyboard_shift));
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
