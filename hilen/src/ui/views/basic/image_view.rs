use std::fmt::Display;

use ui_proc::view;

use super::image_cut::ImageCut;
use crate::{
    deps::refs::{Weak, weak_from_ref},
    gm::{
        LossyConvert,
        flat::{Rect, Size},
    },
    ui::{
        CutAxis, NineSegmentImageView, Setup, Tinted, UIColor, View, ViewCallbacks, ViewData, ViewFrame,
        ViewSubviews,
    },
    window::image::{Image, ToImage},
};

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
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

    cut: Option<ImageCut>,

    /// The name and the tint of an icon set with `set_icon`, kept to tint it
    /// again when the theme changes.
    icon: Option<(String, UIColor)>,

    pub mode: ImageMode,

    pub flip_x: bool,
    pub flip_y: bool,
}

impl ImageView {
    pub(crate) fn image(&self) -> Weak<Image> {
        self.image
    }

    pub fn set_image(&self, image: impl ToImage) -> &Self {
        let mut this = weak_from_ref(self);
        this.icon = None;
        this.image = image.to_image();
        self
    }

    /// Shows a black SVG of `assets/images`, like a Lucide icon, in `tint`.
    /// A `DynamicColor` tint follows the theme, the icon is tinted again on
    /// every theme switch.
    pub fn set_icon(&self, name: impl Display, tint: impl Into<UIColor>) -> &Self {
        let mut this = weak_from_ref(self);
        let name = name.to_string();
        let tint = tint.into();
        this.image = tinted(&name, tint);
        this.icon = Some((name, tint));
        self
    }

    /// Fits a long image to a view of any length with nothing stretched. The
    /// image scales as a whole to the size of the view across `axis`, so a
    /// bigger view gets bigger ends. The first `start` and the last `end`
    /// pixels of the image along `axis` always show, and the middle is cut: a
    /// short view shows a short piece of it, a long view a longer piece. The
    /// piece is taken next to the first end, so only the last end meets a
    /// cut, and the picture has to be even along its length for that cut to
    /// join. A view longer than the image stretches the middle. `mode`, the
    /// flips, the corner radii and the border of the view do not apply.
    pub fn set_cut(&self, axis: CutAxis, start: f32, end: f32) -> &Self {
        weak_from_ref(self).cut = Some(ImageCut { axis, start, end });
        self
    }

    pub(crate) fn cut(&self) -> Option<ImageCut> {
        self.cut
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

    /// The size the picture takes inside `bounds` with its aspect kept, as
    /// big as fits. A frame of this size shows the picture with no blank band,
    /// and its origin is where the picture starts, so a column of pictures
    /// lines up on its left edge. Zero before a picture is set.
    pub fn fit_size(&self, bounds: impl Into<Size>) -> Size {
        if !self.image.is_ok() {
            return Size::default();
        }
        fit_size(self.image.size.into(), bounds.into())
    }

    pub(crate) fn image_frame(&self) -> Rect {
        match self.mode {
            ImageMode::Fill | ImageMode::AspectFill => self.__base_view().draw_frame(),
            ImageMode::AspectFit => self.__base_view().draw_frame().fit_aspect_ratio(self.image.size.into()),
        }
    }

    /// Pixels the image covers on screen, the size an svg rasterizes
    /// at. `AspectFill` scales the image past the frame and crops, so
    /// its raster is the scaled image, not the frame.
    pub(crate) fn raster_size(&self, scale: f32) -> Size<u32> {
        let size = match self.mode {
            ImageMode::Fill | ImageMode::AspectFit => self.image_frame().size,
            ImageMode::AspectFill => {
                let frame = self.frame().size;
                let image: Size = self.image.size.into();
                let ratio = f32::max(frame.width / image.width, frame.height / image.height);
                image * ratio
            }
        };
        Size::new(
            (size.width * scale).round().lossy_convert(),
            (size.height * scale).round().lossy_convert(),
        )
    }

    /// The size an svg of this view rasterizes at, in pixels. A cut image
    /// rasterizes whole at the scale of the view, its quads then show
    /// parts of that raster.
    pub(crate) fn svg_raster_size(&self, scale: f32) -> Size<u32> {
        let Some(cut) = self.cut else {
            return self.raster_size(scale);
        };
        let image_size: Size = self.image.size.into();
        let natural = cut.scaled_size(self.__base_view().draw_frame(), image_size) * scale;
        Size::new(
            natural.width.round().max(1.0).lossy_convert(),
            natural.height.round().max(1.0).lossy_convert(),
        )
    }

    /// Texture subrect in 0..1 UV space. `AspectFill` keeps the quad at the
    /// view frame and crops here instead of enlarging the quad, so corner
    /// radii apply to the visible rect and no scissor is needed.
    pub(crate) fn uv_rect(&self) -> Rect {
        match self.mode {
            ImageMode::Fill | ImageMode::AspectFit => (0, 0, 1, 1).into(),
            ImageMode::AspectFill => {
                let frame = self.frame().size;
                let image: Size = self.image.size.into();
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

fn fit_size(image: Size, bounds: Size) -> Size {
    if image.width <= 0.0 || image.height <= 0.0 || bounds.width <= 0.0 || bounds.height <= 0.0 {
        return Size::default();
    }
    let scale = f32::min(bounds.width / image.width, bounds.height / image.height);
    Size::new(image.width * scale, image.height * scale)
}

fn tinted(name: &str, tint: UIColor) -> Weak<Image> {
    Tinted {
        tint: tint.resolve(),
        name: name.to_string(),
    }
    .to_image()
}

impl ViewCallbacks for ImageView {
    fn theme_changed(&mut self) {
        if let Some((name, tint)) = &self.icon {
            self.image = tinted(name, *tint);
        }
    }
}

impl Setup for ImageView {}

#[cfg(test)]
mod tests {
    use super::fit_size;
    use crate::gm::flat::Size;

    fn near(size: Size, width: f32, height: f32) -> bool {
        (size.width - width).abs() < 0.01 && (size.height - height).abs() < 0.01
    }

    #[test]
    fn a_wide_picture_takes_the_width() {
        let size = fit_size(Size::new(1920.0, 1000.0), Size::new(680.0, 320.0));
        assert!(near(size, 614.4, 320.0), "{size:?}");
        let size = fit_size(Size::new(1920.0, 600.0), Size::new(680.0, 320.0));
        assert!(near(size, 680.0, 212.5), "{size:?}");
    }

    #[test]
    fn a_tall_picture_takes_the_height() {
        let size = fit_size(Size::new(400.0, 800.0), Size::new(680.0, 320.0));
        assert!(near(size, 160.0, 320.0), "{size:?}");
    }

    #[test]
    fn nothing_to_fit_is_zero() {
        assert_eq!(
            fit_size(Size::new(0.0, 0.0), Size::new(680.0, 320.0)),
            Size::default()
        );
        assert_eq!(
            fit_size(Size::new(100.0, 50.0), Size::new(0.0, 320.0)),
            Size::default()
        );
    }
}
