use crate::ui::View;

pub trait CellCallbacks {
    fn cell_added(&mut self) {}
    fn cell_removed(&mut self);
}

impl<T: ?Sized + View> CellCallbacks for T {
    default fn cell_added(&mut self) {}
    default fn cell_removed(&mut self) {}
}
