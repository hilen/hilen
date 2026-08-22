use std::{
    any::type_name,
    fmt::Debug,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use log::error;
use parking_lot::Mutex;

type BoxedFn<In, Out> = Box<dyn FnMut(In) -> Out + Send>;

pub struct Function<In, Out> {
    is_empty: AtomicBool,
    fun:      Arc<Mutex<BoxedFn<In, Out>>>,
}

impl<In, Out> Default for Function<In, Out> {
    fn default() -> Self {
        Self {
            is_empty: AtomicBool::new(true),
            fun:      Arc::new(Mutex::new(Box::new(|_| {
                error!("Calling empty function of type: {}", type_name::<Self>());
                panic!("Calling empty function of type: {}", type_name::<Self>())
            }))),
        }
    }
}

impl<In, Out> Function<In, Out> {
    pub fn new(fun: impl FnMut(In) -> Out + Send + 'static) -> Self {
        Self {
            is_empty: AtomicBool::new(false),
            fun:      Arc::new(Mutex::new(Box::new(fun))),
        }
    }

    pub fn replace(&self, fun: impl FnMut(In) -> Out + Send + 'static) {
        *self.fun.lock() = Box::new(fun);
        self.is_empty.store(false, Ordering::Release);
    }

    pub fn call(&self, input: In) -> Out {
        (*self.fun.lock())(input)
    }

    pub fn is_empty(&self) -> bool {
        self.is_empty.load(Ordering::Acquire)
    }
}

impl<In, Out> Clone for Function<In, Out> {
    fn clone(&self) -> Self {
        Self {
            is_empty: AtomicBool::new(self.is_empty()),
            fun:      self.fun.clone(),
        }
    }
}

impl<In, Out> Debug for Function<In, Out> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        format!("Function<{}, {}>", type_name::<In>(), type_name::<Out>()).fmt(f)
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashSet;

    use crate::deps::netrun::Function;

    #[test]
    fn test_function() {
        let fun = Function::<String, HashSet<i32>>::default();

        assert!(fun.is_empty());

        assert_eq!(
            "\"Function<alloc::string::String, std::collections::hash::set::HashSet<i32>>\"",
            format!("{fun:?}")
        );

        fun.replace(|_| HashSet::new());

        assert!(!fun.is_empty());
    }
}
