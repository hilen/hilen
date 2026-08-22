use crate::deps::refs::{Weak, manage::Managed};

pub trait ExistsManaged {
    fn exists_managed(&self) -> bool;
}

impl<T: Managed> ExistsManaged for Weak<T> {
    fn exists_managed(&self) -> bool {
        self.was_initialized()
    }
}
