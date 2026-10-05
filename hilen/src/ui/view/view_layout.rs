use crate::{
    gm::flat::Point,
    ui::{
        View,
        view::{view_frame::ViewFrame, view_internal::ViewInternal, view_subviews::ViewSubviews},
    },
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
        let superview = *self.superview();
        let (shift, scale) = if superview.is_ok() {
            let above = superview.__base_view();
            (
                above.content_shift * above.tree_scale,
                above.tree_scale * above.content_scale,
            )
        } else {
            (Point::default(), 1.0)
        };
        self.__base_view().tree_scale = scale;

        // The plain path stays as it was, so a view outside a scaled
        // subtree lands on the same bits as before.
        if scale.to_bits() == 1.0_f32.to_bits() && shift == Point::default() {
            self.__base_view().__absolute_frame = *self.frame();
            let orig = self.super_absolute_frame().origin;
            self.__base_view().__absolute_frame.origin += orig;
            let offset = self.__base_view().__content_offset;
            self.__base_view().__absolute_frame.origin.y += offset;
            let offset_x = self.__base_view().__content_offset_x;
            self.__base_view().__absolute_frame.origin.x += offset_x;
            return;
        }

        let orig = self.super_absolute_frame().origin;
        let base = self.__base_view();
        let local = Point::new(
            base.frame.origin.x + base.__content_offset_x,
            base.frame.origin.y + base.__content_offset,
        );
        base.__absolute_frame.origin = orig + shift + local * scale;
        base.__absolute_frame.size = base.frame.size * scale;
    }
}
