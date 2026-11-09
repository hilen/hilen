use crate::{
    gm::flat::Size,
    ui::{View, view::view_frame::ViewFrame},
};

pub trait ViewCallbacks {
    fn update(&mut self);
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
    default fn layout_size(&self) -> &Size {
        &self.frame().size
    }
    default fn theme_changed(&mut self) {}
}
