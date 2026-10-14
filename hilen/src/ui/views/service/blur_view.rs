use ui_proc::view;

use crate::gm::ToF32;

/// Shows a blurred copy of everything drawn before it in tree order,
/// inside its own frame and corner radii. Its color is a tint mixed
/// over the blur, transparent by default. Subviews draw on top of the
/// blur, crisp. Views drawn after it in tree order are not blurred.
/// With radius zero it behaves like a plain view.
#[view]
pub struct BlurView {
    blur_radius: f32,
    tail:        Option<BlurTail>,
}

/// A triangle on the top or bottom edge of a [`BlurView`], like the arrow
/// of a menu that points at its button. It lies inside the frame of the
/// view: the rounded rect takes the frame less `height` on that edge, the
/// triangle takes the rest, and both show the same blur and border. It
/// draws only with a blur radius above zero.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BlurTail {
    pub edge:   TailEdge,
    /// The middle of the triangle, from the left edge of the view.
    pub x:      f32,
    pub width:  f32,
    pub height: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TailEdge {
    Top,
    Bottom,
}

impl BlurView {
    pub(crate) fn blur_radius(&self) -> f32 {
        self.blur_radius
    }

    pub fn set_blur_radius(&mut self, radius: impl ToF32) -> &mut Self {
        self.blur_radius = radius.to_f32();
        self
    }

    pub fn tail(&self) -> Option<BlurTail> {
        self.tail
    }

    pub fn set_tail(&mut self, tail: Option<BlurTail>) -> &mut Self {
        self.tail = tail;
        self
    }

    /// The tail as `ui_backdrop.wgsl` reads it, see `UIBackdropInstance`.
    pub(crate) fn tail_params(&self) -> [f32; 4] {
        let Some(tail) = self.tail else {
            return [0.0; 4];
        };
        let edge = match tail.edge {
            TailEdge::Top => -1.0,
            TailEdge::Bottom => 1.0,
        };
        [tail.x, tail.width / 2.0, tail.height, edge]
    }
}
