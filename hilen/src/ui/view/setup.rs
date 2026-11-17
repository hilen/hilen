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

#[cfg(all(test, feature = "checks"))]
mod tests {
    use std::ops::Deref;

    use serial_test::serial;

    use crate::{
        self as hilen,
        deps::hreads::set_current_thread_as_main,
        ui::{Container, Setup, view},
    };

    #[view]
    struct Holder {
        #[init]
        inner: Container,
    }

    /// The control center called a method on its `#[init]` tab bar before
    /// `set_view` and its page died with "never initialized weak pointer".
    #[test]
    #[serial]
    #[should_panic(expected = "The #[init] view Holder.inner is used before its view was added")]
    fn an_init_view_used_before_the_add_names_itself() {
        set_current_thread_as_main();
        // Leaked, a view dropped while the test unwinds panics a second time.
        let holder = Box::leak(Box::new(Holder::new()));
        let _ = holder.inner.deref();
    }
}
