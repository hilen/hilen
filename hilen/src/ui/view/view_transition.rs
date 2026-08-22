use crate::{deps::refs::Weak, ui::View};

pub trait ViewTransition<To: View + Sized + 'static> {
    fn transition_to(self: Weak<Self>, _target: &mut To);
}

impl<T: View + ?Sized, To: View + Sized + 'static> ViewTransition<To> for T {
    default fn transition_to(self: Weak<Self>, _target: &mut To) {}
}
