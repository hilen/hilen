use crate::deps::refs::{Own, Weak};

pub type OwnVec<T> = Vec<Own<T>>;
pub type WeakVec<T> = Vec<Weak<T>>;

pub trait WeakVecHelper<T: ?Sized> {
    fn remove_freed(&mut self);
}

impl<T: ?Sized> WeakVecHelper<T> for WeakVec<T> {
    fn remove_freed(&mut self) {
        self.retain(Weak::is_ok);
    }
}

#[cfg(test)]
mod test {
    use serial_test::serial;

    use crate::deps::{
        hreads::set_current_thread_as_main,
        refs::{Own, vec::OwnVec},
    };

    fn into_own<T: 'static>(vec: Vec<T>) -> OwnVec<T> {
        vec.into_iter().map(Own::new).collect()
    }

    #[test]
    #[serial]
    fn test_own_vec() {
        set_current_thread_as_main();
        let vec: Vec<u32> = vec![1, 2, 3, 4, 5];
        let mut owned_vec: OwnVec<u32> = into_own(vec.clone());
        let owned_vec2: OwnVec<u32> = into_own(vec);

        assert_eq!(owned_vec, owned_vec2);

        assert_eq!(owned_vec[3], 4);

        assert_eq!(owned_vec.pop().unwrap(), 5);
        set_current_thread_as_main();
    }
}
