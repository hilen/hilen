use std::sync::atomic::{AtomicBool, Ordering};

use crate::{
    gm::flat::Size,
    ui::{View, view::view_frame::ViewFrame},
    window::RenderPass,
};

/// Set by the `before_render` that does nothing. One that a view wrote
/// itself leaves it as it was, and that is how the drawer tells them apart.
static PLAIN_RENDER: AtomicBool = AtomicBool::new(false);

/// Runs the `before_render` of a view and tells whether the view has one
/// of its own.
pub(crate) fn renders_itself(before_render: impl FnOnce()) -> bool {
    PLAIN_RENDER.store(false, Ordering::Relaxed);
    before_render();
    !PLAIN_RENDER.load(Ordering::Relaxed)
}

pub trait ViewCallbacks {
    fn update(&mut self);
    fn before_render(&self, pass: &mut RenderPass);
    /// The size this view's children lay out against. The frame size
    /// for every view, the scroll content overrides it with its own
    /// content size. Not a text measure, `Label::content_size` and
    /// `Button::content_size` are those and only exist where they
    /// mean something.
    fn layout_size(&self) -> &Size;
    fn theme_changed(&mut self);
}

impl<T: ?Sized + View> ViewCallbacks for T {
    default fn update(&mut self) {}
    default fn before_render(&self, _pass: &mut RenderPass) {
        PLAIN_RENDER.store(true, Ordering::Relaxed);
    }
    default fn layout_size(&self) -> &Size {
        &self.frame().size
    }
    default fn theme_changed(&mut self) {}
}
