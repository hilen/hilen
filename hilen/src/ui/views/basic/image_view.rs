use std::fmt::Display;

use ui_proc::view;

use crate::{
    deps::refs::{Weak, weak_from_ref},
    gm::flat::Rect,
    ui::{NineSegmentImageView, Setup, ViewData, ViewFrame, ViewSubviews},
    window::image::{Image, ToImage},
};

#[derive(Default)]
pub enum ImageMode {
    #[default]
    Fill,
    AspectFit,
    AspectFill,
}

#[view]
pub struct ImageView {
    image: Weak<Image>,

    nine_segment: Weak<NineSegmentImageView>,

    pub mode: ImageMode,

    pub flip_x: bool,
    pub flip_y: bool,
}

impl ImageView {
    pub(crate) fn image(&self) -> Weak<Image> {
        self.image
    }

    pub fn set_image(&self, image: impl ToImage) -> &Self {
        weak_from_ref(self).image = image.to_image();
        self
    }

    pub fn set_resizing_image(&mut self, name: impl Display) -> &mut Self {
        if !self.nine_segment.was_initialized() {
            self.nine_segment = self.add_view();
            self.nine_segment.place().back();
            self.nine_segment
                .subviews_weak()
                .iter_mut()
                .for_each(|v| v.__base_view().z_position = self.z_position());
        }

        self.nine_segment.set_image(name);

        self
    }

    pub(crate) fn image_frame(&self) -> Rect {
        match self.mode {
            ImageMode::Fill | ImageMode::AspectFill => *self.absolute_frame(),
            ImageMode::AspectFit => self.absolute_frame().fit_aspect_ratio(self.image.size.into()),
        }
    }

    /// Texture subrect in 0..1 UV space. `AspectFill` keeps the quad at the
    /// view frame and crops here instead of enlarging the quad, so corner
    /// radii apply to the visible rect and no scissor is needed.
    pub(crate) fn uv_rect(&self) -> Rect {
        match self.mode {
            ImageMode::Fill | ImageMode::AspectFit => (0, 0, 1, 1).into(),
            ImageMode::AspectFill => {
                let frame = self.frame().size;
                let image: crate::gm::flat::Size = self.image.size.into();
                let scale = f32::max(frame.width / image.width, frame.height / image.height);
                let uv_width = frame.width / (image.width * scale);
                let uv_height = frame.height / (image.height * scale);
                (
                    (1.0 - uv_width) / 2.0,
                    (1.0 - uv_height) / 2.0,
                    uv_width,
                    uv_height,
                )
                    .into()
            }
        }
    }
}

impl Setup for ImageView {}
