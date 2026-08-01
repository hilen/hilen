use std::{
    any::type_name,
    cell::RefCell,
    fmt::{Debug, Formatter},
};

type Callback<T> = Box<dyn FnMut(T) + Send + 'static>;

pub struct Event<T = ()> {
    subscriber: RefCell<Option<Callback<T>>>,
}

impl<T: 'static> Event<T> {
    pub const fn const_default() -> Self {
        Self {
            subscriber: RefCell::new(None),
        }
    }

    fn check_empty(&self) {
        assert!(
            self.subscriber.borrow().is_none(),
            "Event already has a subscriber"
        );
    }

    pub fn sub(&self, mut action: impl FnMut() + Send + 'static) {
        self.check_empty();
        self.subscriber.replace(Some(Box::new(move |_| {
            action();
        })));
    }

    pub fn val(&self, action: impl FnMut(T) + Send + 'static) {
        self.check_empty();
        self.subscriber.replace(Some(Box::new(action)));
    }

    pub fn trigger(&self, value: T) {
        if let Some(sub) = self.subscriber.borrow_mut().as_mut() {
            (sub)(value);
        }
    }

    pub fn remove_subscribers(&self) {
        self.subscriber.replace(None);
    }
}

impl<T: Send + 'static> Event<T> {
    pub fn val_async<Fut, Function>(&self, action: Function)
    where
        T: Send + 'static,
        Fut: Future + Send + 'static,
        Function: (FnMut(T) -> Fut) + Send + Clone + 'static, {
        self.val(move |val| {
            let mut a = action.clone();
            hreads::spawn(async move {
                a(val).await;
            });
        });
    }
}

impl<T> Default for Event<T> {
    fn default() -> Self {
        Self {
            subscriber: RefCell::default(),
        }
    }
}

impl<T> Debug for Event<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Event<{}>", type_name::<T>())
    }
}
