use log::warn;
use ui_proc::view;

use super::crop_view::{CropShape, ImageCropView};
use crate::{
    self as hilen,
    deps::{refs::Weak, vents::OnceEvent},
    gm::{
        color::{BLACK, CLEAR, WHITE},
        flat::Size,
    },
    ui::{
        Anchor::Width, Button, Container, DialogStyle, Label, ModalView, Setup, Slider, TextAlignment,
        UIColor, UIManager, ViewData, ViewFrame,
    },
};

/// The widest the prompt opens, a phone gets the width of its screen.
const WIDTH: f32 = 420.0;
const PAD: f32 = 16.0;
const TITLE_HEIGHT: f32 = 52.0;
const ZOOM_HEIGHT: f32 = 48.0;
const BUTTON_HEIGHT: f32 = 50.0;
/// Room between the prompt and the edge of the window.
const EDGE: f32 = 12.0;

/// What `ImageCropPrompt` opens with.
#[derive(Debug, Clone)]
pub struct ImageCropRequest {
    /// The bytes of the photo, a png, a jpeg, a gif or a webp, like the
    /// `bytes` of `Paths::pick_image`.
    pub photo:  Vec<u8>,
    /// The side of the png the prompt gives back, in pixels.
    pub side:   u32,
    pub shape:  CropShape,
    pub title:  String,
    pub cancel: String,
    pub save:   String,
}

impl ImageCropRequest {
    pub fn new(photo: Vec<u8>, side: u32) -> Self {
        Self {
            photo,
            side,
            shape: CropShape::Circle,
            title: "Crop the photo".to_owned(),
            cancel: "Cancel".to_owned(),
            save: "Save".to_owned(),
        }
    }

    pub fn shape(mut self, shape: CropShape) -> Self {
        self.shape = shape;
        self
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// The texts of the 2 buttons.
    pub fn buttons(mut self, cancel: impl Into<String>, save: impl Into<String>) -> Self {
        self.cancel = cancel.into();
        self.save = save.into();
        self
    }
}

/// A modal to crop a photo for an avatar: the photo under a round or square
/// mask, a zoom row with `-`, a slider and `+`, then Cancel and Save. It
/// gives back the png of the cut, a square of the side asked for, or None.
/// Escape cancels. The colors come from `DialogStyle`, like `Question`.
///
/// ```ignore
/// let request = ImageCropRequest::new(picked.bytes, 256);
/// ImageCropPrompt::show_modally_with_input(request, move |png| { ... });
/// ```
#[view]
pub struct ImageCropPrompt {
    side:    u32,
    /// The slider and the view move each other, this stops the echo.
    syncing: bool,
    event:   OnceEvent<Option<Vec<u8>>>,

    #[init]
    title:            Label,
    crop:             ImageCropView,
    status:           Label,
    smaller:          Button,
    zoom:             Slider,
    bigger:           Button,
    separator:        Container,
    button_separator: Container,
    cancel:           Button,
    save:             Button,
}

impl ModalView<ImageCropRequest, Option<Vec<u8>>> for ImageCropPrompt {
    fn modal_event(&self) -> &OnceEvent<Option<Vec<u8>>> {
        &self.event
    }

    fn modal_size() -> Size {
        prompt_size(WIDTH)
    }

    fn modal_scrim_color() -> UIColor {
        BLACK.with_alpha(0.4).into()
    }

    fn modal_cancel(self: Weak<Self>) -> Option<Option<Vec<u8>>> {
        Some(None)
    }

    fn setup_input(mut self: Weak<Self>, request: ImageCropRequest) {
        // `show_modally` placed it at `modal_size` after `setup`, a phone
        // screen is narrower.
        self.set_modal_size(prompt_size(fitted_width(UIManager::root_view().size())));
        self.side = request.side;
        self.title.set_text(request.title);
        self.cancel.set_text(request.cancel);
        self.save.set_text(request.save);
        self.crop.set_shape(request.shape);
        self.crop.set_photo(request.photo);
    }
}

impl ImageCropPrompt {
    fn photo_loaded(self: Weak<Self>, ok: bool) {
        self.status.set_hidden(ok);
        if !ok {
            self.status.set_text("This photo could not be opened.");
        }
        self.save.set_enabled(ok);
        self.sync_slider();
    }

    fn sync_slider(mut self: Weak<Self>) {
        self.syncing = true;
        let zoom = self.crop.zoom();
        self.zoom.set_value(zoom);
        self.syncing = false;
    }

    fn slid(mut self: Weak<Self>, zoom: f32) {
        if self.syncing {
            return;
        }
        self.syncing = true;
        self.crop.set_zoom(zoom);
        self.syncing = false;
    }

    fn step_zoom(self: Weak<Self>, factor: f32) {
        let zoom = self.crop.zoom() * factor;
        self.crop.set_zoom(zoom);
    }

    fn save_cut(self: Weak<Self>) {
        match self.crop.cut_png(self.side) {
            Ok(png) => self.hide_modal(Some(png)),
            Err(error) => {
                warn!("Failed to cut the photo: {error:#}");
                let status = self.status;
                status.set_text("This photo could not be cut.");
                status.set_hidden(false);
            }
        }
    }
}

/// The size of the prompt `width` points wide, the crop view is square.
fn prompt_size(width: f32) -> Size {
    let crop = width - PAD * 2.0;
    Size::new(
        width,
        TITLE_HEIGHT + crop + ZOOM_HEIGHT + PAD / 2.0 + BUTTON_HEIGHT,
    )
}

/// The widest prompt that fits the window both ways.
fn fitted_width(window: Size) -> f32 {
    let room_across = window.width - EDGE * 2.0;
    let fixed = prompt_size(WIDTH).height - (WIDTH - PAD * 2.0);
    let room_down = window.height - EDGE * 2.0 - fixed + PAD * 2.0;
    WIDTH.min(room_across).min(room_down).max(160.0)
}

impl Setup for ImageCropPrompt {
    fn setup(self: Weak<Self>) {
        let style = DialogStyle::current();
        self.set_corner_radius(14);
        self.set_color(style.background);

        let width = fitted_width(UIManager::root_view().size());
        let crop_side = width - PAD * 2.0;

        self.title.set_text_size(17).set_text_color(style.text);
        self.title.set_alignment(TextAlignment::Center);
        self.title.place().lrt(0).h(TITLE_HEIGHT);

        // The status text is white over it, the same in both themes.
        self.crop.set_color(BLACK);
        self.crop.set_corner_radius(8);
        self.crop.place().t(TITLE_HEIGHT).l(PAD).size(crop_side, crop_side);
        self.crop.loaded.val(move |ok| self.photo_loaded(ok));
        self.crop.on_change.sub(move || {
            if !self.syncing {
                self.sync_slider();
            }
        });

        self.status
            .set_text("Opening the photo")
            .set_text_size(15)
            .set_text_color(WHITE);
        self.status.set_multiline(true);
        self.status.place().t(TITLE_HEIGHT).l(PAD).size(crop_side, crop_side);

        let zoom_top = TITLE_HEIGHT + crop_side;
        for (button, text, factor) in [(self.smaller, "−", 1.0 / 1.25), (self.bigger, "+", 1.25)] {
            button.set_text(text).set_text_size(26).set_text_color(style.button);
            button.set_color(CLEAR);
            button.on_tap(move || self.step_zoom(factor));
        }
        self.smaller.place().t(zoom_top).l(PAD).size(44, ZOOM_HEIGHT);
        self.bigger.place().t(zoom_top).r(PAD).size(44, ZOOM_HEIGHT);

        self.zoom.set_horizontal();
        self.zoom.set_track_thickness(4).set_thumb_radius(11);
        self.zoom.set_track_color(style.separator).set_fill_color(style.button);
        self.zoom.set_range(1.0, ImageCropView::max_zoom());
        self.zoom.place().t(zoom_top).l(PAD + 52.0).r(PAD + 52.0).h(ZOOM_HEIGHT);
        self.zoom.on_change.val(move |zoom| self.slid(zoom));

        self.separator.set_color(style.separator);
        self.separator.place().lr(0).b(BUTTON_HEIGHT).h(1);
        self.button_separator.set_color(style.separator);
        self.button_separator.place().b(0).w(1).h(BUTTON_HEIGHT).center_x();

        for button in [self.cancel, self.save] {
            button.set_text_size(17).set_text_color(style.button);
            button.set_color(CLEAR);
        }
        self.cancel.place().h(BUTTON_HEIGHT).bl(0).relative(Width, self, 0.5);
        self.cancel.on_tap(move || self.hide_modal(None));
        self.save.place().h(BUTTON_HEIGHT).br(0).relative(Width, self, 0.5);
        self.save.on_tap(move || self.save_cut());
        self.save.set_enabled(false);
    }
}

#[cfg(feature = "ui-tests")]
#[path = "crop_prompt_test.rs"]
mod test;
