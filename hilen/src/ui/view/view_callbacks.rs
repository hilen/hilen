use crate::{
    gm::flat::Size,
    ui::{View, view::view_frame::ViewFrame},
    window::RenderPass,
};

pub trait ViewCallbacks {
    fn update(&mut self);
    fn before_render(&self, pass: &mut RenderPass);
    fn content_size(&self) -> &Size;
    fn theme_changed(&mut self);
}

impl<T: ?Sized + View> ViewCallbacks for T {
    default fn update(&mut self) {}
    default fn before_render(&self, _pass: &mut RenderPass) {}
    default fn content_size(&self) -> &Size {
        &self.frame().size
    }
    default fn theme_changed(&mut self) {}
}
