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
}

impl BlurView {
    pub(crate) fn blur_radius(&self) -> f32 {
        self.blur_radius
    }

    pub fn set_blur_radius(&mut self, radius: impl ToF32) -> &mut Self {
        self.blur_radius = radius.to_f32();
        self
    }
}
