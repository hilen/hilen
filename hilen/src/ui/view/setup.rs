use std::any::type_name;

use crate::{
    deps::refs::{Own, Weak},
    ui::View,
};

pub trait __ViewInternalSetup {
    fn __internal_before_setup(&mut self);
    fn __internal_setup(&mut self);
    fn __internal_inspect(&mut self);
    fn __internal_on_selection_changed(&mut self, selected: bool);
    fn __internal_clips_to_bounds(&self) -> bool;
}

pub trait Setup {
    fn new() -> Own<Self>
    where Self: Default;
    fn setup(self: Weak<Self>);
    fn clips_to_bounds(&self) -> bool;
    fn on_selection_changed(self: Weak<Self>, selected: bool);
    fn before_setup(self: Weak<Self>);
    fn inspect(self: Weak<Self>);
}

impl<T: View + 'static> Setup for T {
    fn new() -> Own<Self>
    where Self: Default {
        let own = Own::<Self>::default();
        own.__base_view().view_label = type_name::<Self>().to_string();
        own
    }

    default fn setup(self: Weak<Self>) {}

    default fn clips_to_bounds(&self) -> bool {
        false
    }

    default fn on_selection_changed(self: Weak<Self>, _selected: bool) {}

    default fn before_setup(self: Weak<Self>) {}
    default fn inspect(self: Weak<Self>) {}
}
