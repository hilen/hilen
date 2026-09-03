use ui_proc::view;

use crate::{
    deps::refs::Weak,
    gm::color::{GRAY, U8Color, WHITE},
    ui::{Anchor, Container, Label, Setup, ViewData, ViewSubviews, WeakView},
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

const RATIO: f32 = 0.1;
const LINE_COLOR: U8Color = U8Color::const_rgb(250, 68, 68);

#[derive(Clone, Copy)]
enum Edge {
    Top,
    Bot,
    Left,
    Right,
}

impl AnchorView {
    fn update_anchor(self: Weak<Self>) {
        self.remove_all_subviews();

        match self.anchor {
            Anchor::Top => {
                self.hor_line();
                self.mark(Edge::Top);
                self.ver_line().place().b(20);
            }
            Anchor::Bot => {
                self.hor_line();
                self.mark(Edge::Bot);
                self.ver_line().place().t(20);
            }
            Anchor::Left => {
                self.ver_line();
                self.mark(Edge::Left);
                self.hor_line().place().r(20);
            }
            Anchor::Right => {
                self.ver_line();
                self.hor_line().place().l(20);
                self.mark(Edge::Right);
            }
            Anchor::Width => self.width(),
            Anchor::Height => self.height(),
            Anchor::MaxWidth => {
                self.width();
                self.max();
            }
            Anchor::MaxHeight => {
                self.height();
                self.max();
            }
            Anchor::Center => {
                self.add_view::<Container>()
                    .set_color(LINE_COLOR)
                    .set_corner_radius(2)
                    .place()
                    .center()
                    .relative_size(self, RATIO * 1.5);
            }
            Anchor::MinWidth
            | Anchor::MinHeight
            | Anchor::CenterX
            | Anchor::CenterY
            | Anchor::X
            | Anchor::Y
            | Anchor::None => {}
        }
    }

    fn hor_line(self: Weak<Self>) -> WeakView {
        self.add_view::<Container>()
            .set_color(LINE_COLOR)
            .place()
            .lr(BORDER_WIDTH)
            .relative_height(self, RATIO)
            .center_y()
            .view()
    }

    fn ver_line(self: Weak<Self>) -> WeakView {
        self.add_view::<Container>()
            .set_color(LINE_COLOR)
            .place()
            .tb(BORDER_WIDTH)
            .relative_width(self, RATIO)
            .center_x()
            .view()
    }

    /// The short bar hugging one edge, three ratios long along it.
    fn mark(self: Weak<Self>, edge: Edge) {
        let bar = self.add_view::<Container>();
        bar.set_color(LINE_COLOR).set_corner_radius(1);
        let placer = bar.place();
        match edge {
            Edge::Top => placer.t(BORDER_WIDTH),
            Edge::Bot => placer.b(BORDER_WIDTH),
            Edge::Left => placer.l(BORDER_WIDTH),
            Edge::Right => placer.r(BORDER_WIDTH),
        };
        match edge {
            Edge::Top | Edge::Bot => {
                placer.relative_width(self, RATIO * 3.0).relative_height(self, RATIO).center_x()
            }
            Edge::Left | Edge::Right => {
                placer.relative_height(self, RATIO * 3.0).relative_width(self, RATIO).center_y()
            }
        };
    }

    fn width(self: Weak<Self>) {
        self.hor_line();
        self.mark(Edge::Left);
        self.mark(Edge::Right);
    }

    fn height(self: Weak<Self>) {
        self.ver_line();
        self.mark(Edge::Top);
        self.mark(Edge::Bot);
    }

    fn max(self: Weak<Self>) {
        self.add_view::<Label>()
            .set_text("M")
            .set_text_size(59)
            .set_corner_radius(20)
            .set_color(WHITE)
            .place()
            .center()
            .relative_size(self, 0.4);
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
