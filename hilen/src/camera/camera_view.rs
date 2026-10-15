use std::sync::atomic::{AtomicU64, Ordering};

use log::debug;
use ui_proc::view;

use crate::{
    camera::{Camera, CameraFrame},
    deps::refs::{Weak, manage::DataManager, weak_from_ref},
    gm::flat::Size,
    ui::{ImageMode, ImageView, Setup, View, ViewCallbacks, ViewData, ViewFrame, keep_frames_exempt},
    window::image::{Image, NoImage, Texture},
};

/// Unique per picture, so 2 views never write into 1 texture.
static NEXT_ID: AtomicU64 = AtomicU64::new(0);

/// Shows the live picture of an open `Camera`, fit inside the view by
/// default. The view holds a clone of the camera, the camera stays open
/// while the view has it.
#[view]
pub struct CameraView {
    camera:  Option<Camera>,
    /// The number of the frame on screen, 0 for none.
    shown:   u64,
    picture: Option<Picture>,

    #[init]
    image_view: ImageView,
}

/// The texture the frames go into. Freed with the view, or when a frame
/// of another size comes.
struct Picture {
    name:    String,
    size:    Size<u32>,
    texture: wgpu::Texture,
}

impl Drop for Picture {
    fn drop(&mut self) {
        Image::free_with_name(&self.name);
    }
}

impl CameraView {
    /// The camera to show, none for an empty view.
    pub fn set_camera(&self, camera: Option<Camera>) -> &Self {
        let mut this = weak_from_ref(self);
        if let Some(camera) = &camera {
            debug!("camera view shows {}", camera.device().name);
        }
        this.camera = camera;
        this.shown = 0;
        if this.camera.is_none() {
            this.image_view.set_image(NoImage);
            this.picture = None;
        }
        this.keep_frames_coming();
        self
    }

    pub fn camera(&self) -> Option<&Camera> {
        self.camera.as_ref()
    }

    /// `AspectFit` by default, `AspectFill` fills the view and cuts the
    /// edges of the picture.
    pub fn set_mode(&self, mode: ImageMode) -> &Self {
        weak_from_ref(self).image_view.mode = mode;
        self
    }

    /// The number of the frame on screen, 0 before the first one.
    pub fn shown_frame(&self) -> u64 {
        self.shown
    }

    /// A camera sends frames on its own, the render on demand loop has to
    /// ask for frames to see them. It is no loop of the view, so a long
    /// look at the camera is no mistake.
    fn keep_frames_coming(self: Weak<Self>) {
        if self.__base_view().keeps_frames {
            return;
        }
        keep_frames_exempt(self.weak_view(), move || {
            self.is_visible_on_screen() && self.camera.is_some()
        });
    }

    fn show(&mut self, frame: &CameraFrame) {
        let size = Size::new(frame.width, frame.height);
        if self.picture.as_ref().is_none_or(|picture| picture.size != size) {
            let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
            let name = format!("camera-{id}-{}x{}", size.width, size.height);
            debug!("camera view picture is {} by {}", size.width, size.height);
            let (image, texture) = Image::stream(&name, size);
            self.image_view.set_image(image);
            self.picture = Some(Picture { name, size, texture });
        }
        if let Some(picture) = &self.picture {
            Texture::write_level_0(&picture.texture, size, &frame.rgba);
        }
        self.shown = frame.number;
    }
}

impl Setup for CameraView {
    fn setup(mut self: Weak<Self>) {
        self.image_view.place().back();
        self.image_view.mode = ImageMode::AspectFit;
    }
}

impl ViewCallbacks for CameraView {
    fn update(&mut self) {
        if !self.is_visible_on_screen() {
            return;
        }
        let Some(frame) = self.camera.as_ref().and_then(Camera::frame) else {
            return;
        };
        if frame.number != self.shown {
            self.show(&frame);
        }
        self.weak().keep_frames_coming();
    }
}
