use crate::ui::{
    View,
    view::{view_frame::ViewFrame, view_internal::ViewInternal},
};

pub trait ViewLayout {
    fn calculate_absolute_frame(&mut self);
    fn layout(&mut self)
    where Self: View {
        self.__base_view().placer.layout();
    }
}

impl<T: ?Sized + View> ViewLayout for T {
    fn calculate_absolute_frame(&mut self) {
        self.__base_view().__absolute_frame = *self.frame();
        let orig = self.super_absolute_frame().origin;
        self.__base_view().__absolute_frame.origin += orig;
        let offset = self.__base_view().__content_offset;
        self.__base_view().__absolute_frame.origin.y += offset;
    }
}
