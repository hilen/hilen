use log::info;
use ui_proc::view;

use super::{ImageViewerPage, button::ViewerButton};
use crate::{
    self as hilen,
    deps::{refs::Weak, vents::OnceEvent},
    gm::{
        LossyConvert,
        color::{BLACK, WHITE},
        flat::{Point, Rect, Size},
    },
    image::Image,
    ui::{
        CanvasView, Container, ContextMenu, ImageMode, ImageView, Label, MenuItem, ModalView, NamedKey,
        Setup, TextAlignment, Touch, UIColor, UIImages, UIManager, ViewData, ViewFocus, ViewFrame,
        ViewSubviews, ViewTouch,
    },
};

/// The fitted picture never comes closer than this to the window edge.
const MARGIN: f32 = 24.0;
const BUTTON: f32 = 44.0;
const BUTTON_EDGE: f32 = 12.0;
/// A view of the app leaves the sides to the step buttons.
const PAGE_SIDE: f32 = BUTTON_EDGE * 2.0 + BUTTON;
const POSITION_WIDTH: f32 = 72.0;
const POSITION_HEIGHT: f32 = 24.0;
const NOTE_WIDTH: f32 = 420.0;
const NOTE_HEIGHT: f32 = 28.0;
/// A finger that moved more than this between down and up panned, it did
/// not tap.
const TAP_SLOP: f32 = 10.0;
/// A double tap on a picture that has few pixels zooms this far.
const DOUBLE_TAP_ZOOM: f32 = 2.5;
/// How far past 1 point a pixel the zoom goes, and the least it goes.
const PAST_REAL: f32 = 2.0;
const LEAST_MAX_ZOOM: f32 = 4.0;

type Opener = Box<dyn FnMut(ImageViewerPage) + Send>;
type Menu = Box<dyn FnMut(usize) -> Vec<MenuItem> + Send>;

/// What `ImageViewer` opens with: how many pages, the one it opens at, and
/// a closure that fills a page each time one opens.
pub struct ImageViewerRequest {
    count: usize,
    at:    usize,
    open:  Opener,
    menu:  Option<Menu>,
}

impl ImageViewerRequest {
    /// `open` gets each page as it opens and fills it, see
    /// `ImageViewerPage`.
    pub fn new(count: usize, at: usize, open: impl FnMut(ImageViewerPage) + Send + 'static) -> Self {
        Self {
            count,
            at,
            open: Box::new(open),
            menu: None,
        }
    }

    /// Pictures the app holds already.
    pub fn images(images: Vec<Weak<Image>>, at: usize) -> Self {
        Self::new(images.len(), at, move |page| {
            if let Some(image) = images.get(page.index()) {
                page.show_image(*image);
            }
        })
    }

    /// Pictures at these addresses, fetched with `Image::download` under
    /// the address as their name. A picture the page shows already under
    /// that name comes with no request.
    #[cfg(feature = "net")]
    pub fn urls(urls: Vec<String>, at: usize) -> Self {
        Self::new(urls.len(), at, move |page| {
            let Some(url) = urls.get(page.index()).cloned() else {
                return;
            };
            crate::deps::hreads::spawn(async move {
                let image = Image::download(&url, &url).await;
                crate::deps::hreads::on_main(move || match image {
                    Ok(image) => page.show_image(image),
                    Err(error) => {
                        log::warn!("The viewer could not load the picture {url}: {error}");
                        page.show_note(format!("The picture did not load: {error}"));
                    }
                });
            });
        })
    }

    /// The items of the menu of a right click or a long press on a page,
    /// like Copy picture. No items, no menu.
    pub fn menu(mut self, menu: impl FnMut(usize) -> Vec<MenuItem> + Send + 'static) -> Self {
        self.menu = Some(Box::new(menu));
        self
    }
}

/// Pictures at full size over the whole window, on a dark backdrop. The
/// picture opens fitted to the window. The wheel and a pinch zoom it, a
/// drag pans it, and a double tap goes between fitted and 1 point a pixel.
/// With more than 1 page the buttons on the sides and the left and right
/// arrows step through them, with "2 / 5" on top. The close button, Escape
/// and a tap beside the picture close it.
///
/// ```ignore
/// ImageViewer::show_modally_with_input(ImageViewerRequest::urls(urls, 2), |()| {});
/// ```
#[view]
pub struct ImageViewer {
    event:            OnceEvent<()>,
    count:            usize,
    pub(super) at:    usize,
    open:             Option<Opener>,
    menu:             Option<Menu>,
    pub(super) leave: Vec<Box<dyn FnOnce() + Send>>,
    /// The picture of the page and whether it is the full one.
    image:            Option<(Weak<Image>, bool)>,
    /// Where the picture sits at zoom 1, in the points of the canvas.
    fitted:           Rect,
    /// Where the finger that is down went down.
    down_at:          Option<Point>,
    picture:          Weak<ImageView>,

    #[init]
    canvas:          CanvasView,
    pub(super) page: Container,
    note:            Label,
    previous:        ViewerButton,
    next:            ViewerButton,
    close:           ViewerButton,
    position:        Label,
}

impl ModalView<ImageViewerRequest, ()> for ImageViewer {
    fn modal_event(&self) -> &OnceEvent<()> {
        &self.event
    }

    fn modal_size() -> Size {
        UIManager::root_view().size()
    }

    fn modal_scrim_color() -> UIColor {
        BLACK.with_alpha(0.88).into()
    }

    fn modal_cancel(self: Weak<Self>) -> Option<()> {
        self.leave_page();
        Some(())
    }

    fn setup_input(mut self: Weak<Self>, request: ImageViewerRequest) {
        // Over the whole window, also after it changes size.
        self.place().clear().back();
        self.count = request.count;
        self.open = Some(request.open);
        self.menu = request.menu;
        info!(
            "The image viewer opens at {} of {}",
            request.at + 1,
            request.count
        );
        self.open_page(request.at);
    }
}

impl Setup for ImageViewer {
    fn setup(mut self: Weak<Self>) {
        self.canvas.place().back();
        self.picture = self.canvas.add_view::<ImageView>();
        self.picture.mode = ImageMode::AspectFit;
        self.picture.set_hidden(true);
        self.canvas.touch().began.val(move |touch: Touch| {
            let mut this = self;
            this.down_at = Some(touch.position);
        });
        self.canvas.touch().up_inside.val(self, move |touch| self.tapped(touch));
        self.canvas.touch().double_tap.val(self, move |touch| self.double_tapped(touch));
        self.canvas.touch().secondary.val(self, move |_| self.show_menu());

        self.page.place().lr(PAGE_SIDE).tb(MARGIN);

        self.note
            .set_text_size(14)
            .set_text_color(WHITE)
            .set_alignment(TextAlignment::Center)
            .set_color(BLACK.with_alpha(0.55))
            .set_corner_radius(NOTE_HEIGHT / 2.0);
        self.note.set_hidden(true);

        for (button, icon, forward) in [
            (self.previous, UIImages::chevron_left(WHITE), false),
            (self.next, UIImages::chevron_right(WHITE), true),
        ] {
            button.set_corner_radius(BUTTON / 2.0);
            button.set_icon(icon);
            button.tapped.sub(move || self.step(forward));
        }
        self.previous.place().l(BUTTON_EDGE).center_y().size(BUTTON, BUTTON);
        self.next.place().r(BUTTON_EDGE).center_y().size(BUTTON, BUTTON);

        self.close.set_corner_radius(BUTTON / 2.0);
        self.close.set_icon(UIImages::x(WHITE));
        self.close.place().t(BUTTON_EDGE).r(BUTTON_EDGE).size(BUTTON, BUTTON);
        self.close.tapped.sub(move || self.close());

        self.position
            .set_text_size(13)
            .set_text_color(WHITE)
            .set_alignment(TextAlignment::Center)
            .set_color(BLACK.with_alpha(0.55))
            .set_corner_radius(POSITION_HEIGHT / 2.0);
        self.position
            .place()
            .center_x()
            .t(BUTTON_EDGE + (BUTTON - POSITION_HEIGHT) / 2.0)
            .size(POSITION_WIDTH, POSITION_HEIGHT);

        // Arrows move the key focus ring while a view can take it, here
        // they step. Escape and the arrows do what the buttons do.
        self.canvas.set_key_focus(false);
        for button in [self.previous, self.next, self.close] {
            button.set_key_focus(false);
        }
        UIManager::keymap().add(self, NamedKey::ArrowLeft, move || self.key_step(false));
        UIManager::keymap().add(self, NamedKey::ArrowRight, move || self.key_step(true));

        self.size_changed().sub(move || self.fit());
    }
}

impl ImageViewer {
    /// The page that shows, from 0.
    pub fn page_index(&self) -> usize {
        self.at
    }

    /// How much bigger than fitted the picture is drawn.
    pub fn zoom(&self) -> f32 {
        self.canvas.zoom()
    }

    fn close(self: Weak<Self>) {
        self.leave_page();
        self.hide_modal(());
    }

    fn leave_page(mut self: Weak<Self>) {
        for action in std::mem::take(&mut self.leave) {
            action();
        }
    }

    fn key_step(self: Weak<Self>, forward: bool) {
        // A menu over the viewer takes the arrows.
        if crate::ui::TouchStack::top_layer_root().raw() == self.raw() {
            self.step(forward);
        }
    }

    fn step(self: Weak<Self>, forward: bool) {
        let wanted = if forward {
            self.at + 1
        } else {
            let Some(before) = self.at.checked_sub(1) else {
                return;
            };
            before
        };
        if wanted < self.count {
            info!("The image viewer steps to {} of {}", wanted + 1, self.count);
            self.open_page(wanted);
        }
    }

    fn open_page(mut self: Weak<Self>, at: usize) {
        self.leave_page();
        self.at = at;
        self.page.remove_all_subviews();
        self.image = None;
        self.picture.set_hidden(true);
        self.note.set_hidden(true);
        self.fitted = Rect::default();
        self.canvas.set_bounds(None);
        self.reset_zoom(LEAST_MAX_ZOOM);

        self.previous.set_hidden(at == 0);
        self.next.set_hidden(at + 1 >= self.count);
        self.position.set_hidden(self.count < 2);
        self.position.set_text(format!("{} / {}", at + 1, self.count));

        let page = ImageViewerPage {
            viewer: self,
            index:  at,
        };
        if let Some(mut open) = self.open.take() {
            open(page);
            if self.is_ok() {
                self.open = Some(open);
            }
        }
    }

    pub(super) fn show(mut self: Weak<Self>, image: Weak<Image>, full: bool) {
        self.image = Some((image, full));
        self.picture.set_image(image);
        self.picture.set_hidden(false);
        self.note.set_hidden(true);
        self.fit();
    }

    pub(super) fn show_note(self: Weak<Self>, text: String) {
        self.note.set_text(text);
        self.note.set_hidden(false);
        self.place_note();
    }

    pub(super) fn hide_note(self: Weak<Self>) {
        self.note.set_hidden(true);
    }

    /// In the middle while nothing shows, at the bottom over a picture.
    fn place_note(self: Weak<Self>) {
        let place = self.note.place();
        place.clear().center_x().size(NOTE_WIDTH.min(self.width()), NOTE_HEIGHT);
        if self.picture.is_hidden() && self.page.subviews().is_empty() {
            place.center_y();
        } else {
            place.b((MARGIN - NOTE_HEIGHT / 2.0).max(4.0));
        }
    }

    /// Fits the picture into the window at zoom 1. A full picture is
    /// never drawn bigger than 1 point a pixel, a preview has no pixels to
    /// go by.
    fn fit(mut self: Weak<Self>) {
        if !self.note.is_hidden() {
            self.place_note();
        }
        let Some((image, full)) = self.image else {
            return;
        };
        let (pixels_wide, pixels_high): (f32, f32) = (
            image.size.width.lossy_convert(),
            image.size.height.lossy_convert(),
        );
        let (wide, high) = (self.width(), self.height());
        if pixels_wide <= 0.0 || pixels_high <= 0.0 || wide <= 0.0 || high <= 0.0 {
            return;
        }
        let room = ((wide - MARGIN * 2.0).max(1.0), (high - MARGIN * 2.0).max(1.0));
        let mut scale = (room.0 / pixels_wide).min(room.1 / pixels_high);
        if full {
            scale = scale.min(1.0);
        }
        let size = Size::new(pixels_wide * scale, pixels_high * scale);
        let fitted = Rect::new(
            (wide - size.width) / 2.0,
            (high - size.height) / 2.0,
            size.width,
            size.height,
        );
        let max_zoom = if full {
            (PAST_REAL / scale).max(LEAST_MAX_ZOOM)
        } else {
            LEAST_MAX_ZOOM
        };
        self.picture.set_frame(fitted);
        // A full picture after its preview takes the same place, the zoom
        // stays.
        if fitted == self.fitted {
            self.canvas.set_zoom_limits(1, max_zoom);
        } else {
            self.fitted = fitted;
            self.canvas.set_bounds(Some(fitted));
            self.reset_zoom(max_zoom);
        }
    }

    fn reset_zoom(mut self: Weak<Self>, max_zoom: f32) {
        self.canvas.set_zoom_limits(1, max_zoom);
        self.canvas.set_zoom(1);
        self.canvas.set_offset(Point::default());
    }

    fn on_picture(self: Weak<Self>, touch: Touch) -> bool {
        !self.picture.is_hidden() && self.fitted.contains(self.canvas.content_point(touch.position))
    }

    fn tapped(mut self: Weak<Self>, touch: Touch) {
        let Some(down) = self.down_at.take() else {
            return;
        };
        if (down - touch.position).length() > TAP_SLOP || self.on_picture(touch) {
            return;
        }
        self.close();
    }

    fn double_tapped(mut self: Weak<Self>, touch: Touch) {
        let Some((image, _)) = self.image else {
            return;
        };
        if !self.on_picture(touch) {
            return;
        }
        if self.canvas.zoom() > 1.01 {
            self.canvas.set_zoom(1);
            return;
        }
        let pixels: f32 = image.size.width.lossy_convert();
        let real = pixels / self.fitted.size.width;
        let zoom = if real >= 1.5 { real } else { DOUBLE_TAP_ZOOM };
        self.canvas.zoom_at(touch.position, zoom);
    }

    fn show_menu(mut self: Weak<Self>) {
        let at = self.at;
        let Some(menu) = self.menu.as_mut() else {
            return;
        };
        let items = menu(at);
        if !items.is_empty() {
            ContextMenu::show_at_cursor(items);
        }
    }
}

#[cfg(feature = "ui-tests")]
#[path = "viewer_test.rs"]
mod test;
