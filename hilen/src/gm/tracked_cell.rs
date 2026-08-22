use std::{
    cell::{Ref, RefCell, RefMut},
    panic::Location,
};

#[derive(Debug, Clone)]
pub struct TrackedCell<T> {
    inner:       RefCell<T>,
    borrowed_at: RefCell<Option<&'static Location<'static>>>,
}

impl<T> TrackedCell<T> {
    pub fn new(val: T) -> Self {
        Self {
            inner:       RefCell::new(val),
            borrowed_at: RefCell::new(None),
        }
    }

    #[track_caller]
    pub(crate) fn borrow(&self) -> Ref<'_, T> {
        if let Ok(b) = self.inner.try_borrow() {
            *self.borrowed_at.try_borrow_mut().expect("TrackedCell WTF") = Some(Location::caller());
            b
        } else {
            let loc = self
                .borrowed_at
                .try_borrow()
                .expect("TrackedCell WTF")
                .expect("Unknown location");
            panic!(
                "Conflicting borrow! Already borrowed at: {}:{}",
                loc.file(),
                loc.line()
            );
        }
    }

    #[track_caller]
    pub(crate) fn borrow_mut(&self) -> RefMut<'_, T> {
        if let Ok(b) = self.inner.try_borrow_mut() {
            *self.borrowed_at.try_borrow_mut().expect("TrackedCell WTF") = Some(Location::caller());
            b
        } else {
            let loc = self
                .borrowed_at
                .try_borrow()
                .expect("TrackedCell WTF")
                .expect("Unknown location");
            panic!(
                "Conflicting borrow! Already borrowed at: {}:{}",
                loc.file(),
                loc.line()
            );
        }
    }
}

impl<T: PartialEq> PartialEq for TrackedCell<T> {
    fn eq(&self, other: &Self) -> bool {
        self.borrow().eq(&other.borrow())
    }
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    #[should_panic(expected = "Conflicting borrow! Already borrowed at: hilen")]
    fn test_tracked_cell() {
        let cell = TrackedCell::new(5);

        let _a = cell.borrow_mut();
        let _b = cell.borrow_mut();
    }
}
