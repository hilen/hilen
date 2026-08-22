use std::fmt::Display;

use ui_proc::view;

use crate::{
    deps::refs::Weak,
    gm::Apply,
    ui::{ImageView, Setup, ViewCallbacks, ViewFrame},
    window::image::Image,
};

#[view]
pub struct NineSegmentImageView {
    _corner: Weak<Image>,
    _side:   Weak<Image>,
    _center: Weak<Image>,

    #[educe(Default = 64.0)]
    pub corner_size: f32,

    #[init]
    tl: ImageView,
    tr: ImageView,
    bl: ImageView,
    br: ImageView,

    t: ImageView,
    b: ImageView,
    r: ImageView,
    l: ImageView,

    c: ImageView,
}

impl NineSegmentImageView {
    pub fn set_image(self: Weak<Self>, name: impl Display) {
        [self.tl, self.tr, self.bl, self.br].apply(|v| {
            v.set_image(format!("{name}/corner.png"));
        });

        [self.t, self.b].apply(|v| {
            v.set_image(format!("{name}/side_v.png"));
        });

        [self.r, self.l].apply(|v| {
            v.set_image(format!("{name}/side_h.png"));
        });

        self.c.set_image(format!("{name}/center.png"));
    }
}

impl Setup for NineSegmentImageView {
    fn setup(mut self: Weak<Self>) {
        self.tr.flip_x = true;
        self.bl.flip_y = true;
        self.br.flip_x = true;
        self.br.flip_y = true;

        self.b.flip_y = true;
        self.r.flip_x = true;
    }
}

impl ViewCallbacks for NineSegmentImageView {
    fn update(&mut self) {
        let w = self.width();
        let h = self.height();

        let mut cs = self.corner_size;

        let smaller_side = if w < h { w } else { h };

        if cs > smaller_side / 2.0 {
            cs = smaller_side / 2.0;
        }

        self.tl.set_frame((0, 0, cs, cs));
        self.tr.set_frame((w - cs, 0, cs, cs));
        self.bl.set_frame((0, h - cs, cs, cs));
        self.br.set_frame((w - cs, h - cs, cs, cs));

        let middle_w = w - cs * 2.0;
        let middle_h = h - cs * 2.0;

        self.t.set_frame((cs, 0, middle_w, cs));
        self.b.set_frame((cs, h - cs, middle_w, cs));
        self.r.set_frame((w - cs, cs, cs, middle_h));
        self.l.set_frame((0, cs, cs, middle_h));

        self.c.set_frame((cs, cs, middle_w, middle_h));
    }
}
