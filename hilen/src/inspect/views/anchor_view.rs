use ui_proc::view;

use crate::{
    deps::refs::Weak,
    gm::color::{GRAY, U8Color, WHITE},
    ui::{Anchor, Container, Label, Setup, ViewData, ViewSubviews},
};

const BORDER_WIDTH: f32 = 2.0;

#[view(crate = crate)]
pub struct AnchorView {
    anchor: Anchor,
}

impl AnchorView {
    pub fn anchor(&self) -> Anchor {
        self.anchor
    }

    pub fn set_anchor(mut self: Weak<Self>, anchor: Anchor) {
        if anchor == self.anchor {
            return;
        }

        self.anchor = anchor;
        self.update_anchor();
    }
}

impl AnchorView {
    #[allow(clippy::match_same_arms)]
    #[allow(clippy::too_many_lines)]
    fn update_anchor(self: Weak<Self>) {
        const RATIO: f32 = 0.1;
        const LINE_COLOR: U8Color = U8Color::const_rgb(250, 68, 68);

        self.remove_all_subviews();

        let hor_line = move || {
            self.add_view::<Container>()
                .set_color(LINE_COLOR)
                .place()
                .lr(BORDER_WIDTH)
                .relative_height(self, RATIO)
                .center_y()
                .view()
        };

        let ver_line = move || {
            self.add_view::<Container>()
                .set_color(LINE_COLOR)
                .place()
                .tb(BORDER_WIDTH)
                .relative_width(self, RATIO)
                .center_x()
                .view()
        };

        let smol_top = move || {
            self.add_view::<Container>()
                .set_color(LINE_COLOR)
                .set_corner_radius(1)
                .place()
                .t(BORDER_WIDTH)
                .relative_width(self, RATIO * 3.0)
                .relative_height(self, RATIO)
                .center_x();
        };

        let smol_bot = move || {
            self.add_view::<Container>()
                .set_color(LINE_COLOR)
                .set_corner_radius(1)
                .place()
                .b(BORDER_WIDTH)
                .relative_width(self, RATIO * 3.0)
                .relative_height(self, RATIO)
                .center_x();
        };

        let smol_l = move || {
            self.add_view::<Container>()
                .set_color(LINE_COLOR)
                .set_corner_radius(1)
                .place()
                .l(BORDER_WIDTH)
                .relative_height(self, RATIO * 3.0)
                .relative_width(self, RATIO)
                .center_y();
        };

        let smol_r = move || {
            self.add_view::<Container>()
                .set_color(LINE_COLOR)
                .set_corner_radius(1)
                .place()
                .r(BORDER_WIDTH)
                .relative_height(self, RATIO * 3.0)
                .relative_width(self, RATIO)
                .center_y();
        };

        let width = move || {
            hor_line();
            smol_l();
            smol_r();
        };

        let height = move || {
            ver_line();
            smol_top();
            smol_bot();
        };

        let max = move || {
            self.add_view::<Label>()
                .set_text("M")
                .set_text_size(59)
                .set_corner_radius(20)
                .set_color(WHITE)
                .place()
                .center()
                .relative_size(self, 0.4);
        };

        match self.anchor {
            Anchor::Top => {
                hor_line();
                smol_top();
                ver_line().place().b(20);
            }
            Anchor::Bot => {
                hor_line();
                smol_bot();
                ver_line().place().t(20);
            }
            Anchor::Left => {
                ver_line();
                smol_l();
                hor_line().place().r(20);
            }
            Anchor::Right => {
                ver_line();
                hor_line().place().l(20);
                smol_r();
            }
            Anchor::Width => width(),
            Anchor::Height => height(),
            Anchor::MaxWidth => {
                width();
                max();
            }
            Anchor::MaxHeight => {
                height();
                max();
            }
            Anchor::MinWidth => {}
            Anchor::MinHeight => {}
            Anchor::CenterX => {}
            Anchor::CenterY => {}
            Anchor::Center => {
                self.add_view::<Container>()
                    .set_color(LINE_COLOR)
                    .set_corner_radius(2)
                    .place()
                    .center()
                    .relative_size(self, RATIO * 1.5);
            }
            Anchor::X => {}
            Anchor::Y => {}
            Anchor::None => {}
        }
    }
}

impl Setup for AnchorView {
    fn setup(self: Weak<Self>) {
        self.set_color(WHITE)
            .set_corner_radius(5)
            .set_border_color(GRAY)
            .set_border_width(BORDER_WIDTH);
    }
}
