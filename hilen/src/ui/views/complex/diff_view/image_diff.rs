use filediff::Blob;

use crate::{
    refs::{Weak, manage::DataManager},
    ui::{
        Container, Image, ImageMode, ImageView, Label, NoImage, Setup, TextAlignment, ViewData, ViewFrame,
        diff_colors as colors, view,
    },
};

// The image diff body: Before and After side by side, each an AspectFit
// image over the panel background with a border, like the original's
// image template. A missing side gives its space to the other one.

const PAD: f32 = 16.0;
const LABEL_H: f32 = 18.0;

#[view]
pub struct ImageDiff {
    has_old: bool,
    has_new: bool,

    #[init]
    before_label: Label,
    before_back:  Container,
    before:       ImageView,
    after_label:  Label,
    after_back:   Container,
    after:        ImageView,
}

impl Setup for ImageDiff {
    fn setup(self: Weak<Self>) {
        for label in [self.before_label, self.after_label] {
            label
                .set_text_size(10)
                .set_text_color(colors::MUTED)
                .set_alignment(TextAlignment::Left);
        }
        self.before_label.set_text("BEFORE");
        self.after_label.set_text("AFTER");
        // The panel background and border live on a backing container.
        // A color on the ImageView itself would draw over its texture,
        // the two pipelines share one depth.
        for back in [self.before_back, self.after_back] {
            back.set_color(colors::BG_PANEL);
            back.set_border_color(colors::BORDER);
        }
        for mut image in [self.before, self.after] {
            image.mode = ImageMode::AspectFit;
        }
        self.size_changed().sub(move || self.layout());
    }
}

impl ImageDiff {
    pub fn set_blobs(mut self: Weak<Self>, old: Option<&Blob>, new: Option<&Blob>) {
        self.has_old = Self::load_into(self.before, old);
        self.has_new = Self::load_into(self.after, new);
        self.layout();
    }

    fn load_into(view: Weak<ImageView>, blob: Option<&Blob>) -> bool {
        let Some(blob) = blob else {
            view.set_image(NoImage);
            return false;
        };
        view.set_image(Image::load(&blob.bytes, &blob.name));
        true
    }

    fn layout(self: Weak<Self>) {
        let w = self.width();
        let h = self.height();
        let image_h = (h - PAD * 2.0 - LABEL_H).max(0.0);
        let both = self.has_old && self.has_new;
        let half = if both {
            (w - PAD * 3.0) / 2.0
        } else {
            w - PAD * 2.0
        };
        let half = half.max(0.0);

        self.before_label.set_hidden(!self.has_old);
        self.before_back.set_hidden(!self.has_old);
        self.before.set_hidden(!self.has_old);
        self.after_label.set_hidden(!self.has_new);
        self.after_back.set_hidden(!self.has_new);
        self.after.set_hidden(!self.has_new);

        // Left aligned label text indents 8 points inside its frame.
        self.before_label.set_frame((PAD - 8.0, PAD, half, LABEL_H));
        self.before_back.set_frame((PAD, PAD + LABEL_H, half, image_h));
        self.before.set_frame((PAD, PAD + LABEL_H, half, image_h));

        let after_x = if both { PAD * 2.0 + half } else { PAD };
        self.after_label.set_frame((after_x - 8.0, PAD, half, LABEL_H));
        self.after_back.set_frame((after_x, PAD + LABEL_H, half, image_h));
        self.after.set_frame((after_x, PAD + LABEL_H, half, image_h));
    }
}
