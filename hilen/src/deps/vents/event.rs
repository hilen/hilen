use std::{
    any::type_name,
    cell::{Cell, RefCell},
    fmt::{Debug, Formatter},
    mem::take,
};

type Callback<T> = Box<dyn FnMut(T) + Send + 'static>;

/// An event with any number of subscribers. Each `sub` or `val` adds one,
/// and `trigger` calls them all in the order they came. An engine view often
/// subscribes to its own events, so an app that subscribes too must not
/// replace or break that.
pub struct Event<T = ()> {
    subscribers: RefCell<Vec<Callback<T>>>,
    // Counts the calls of `remove_subscribers`, so a removal from inside a
    // callback is not undone when `trigger` puts its list back.
    removals:    Cell<u64>,
}

impl<T: 'static> Event<T> {
    pub const fn const_default() -> Self {
        Self {
            subscribers: RefCell::new(Vec::new()),
            removals:    Cell::new(0),
        }
    }

    pub fn sub(&self, mut action: impl FnMut() + Send + 'static) {
        self.subscribers.borrow_mut().push(Box::new(move |_| {
            action();
        }));
    }

    pub fn val(&self, action: impl FnMut(T) + Send + 'static) {
        self.subscribers.borrow_mut().push(Box::new(action));
    }

    pub fn remove_subscribers(&self) {
        self.removals.set(self.removals.get() + 1);
        self.subscribers.borrow_mut().clear();
    }

    pub fn has_subscribers(&self) -> bool {
        !self.subscribers.borrow().is_empty()
    }
}

impl<T: Clone + 'static> Event<T> {
    pub fn trigger(&self, value: T) {
        // The list is taken out while the callbacks run, so a callback can
        // subscribe, remove or trigger this same event without a borrow panic.
        let mut running = take(&mut *self.subscribers.borrow_mut());
        if running.is_empty() {
            return;
        }
        let removals = self.removals.get();
        let last = running.len() - 1;
        for sub in &mut running[..last] {
            sub(value.clone());
        }
        running[last](value);

        let mut subscribers = self.subscribers.borrow_mut();
        if self.removals.get() != removals {
            return;
        }
        let added = take(&mut *subscribers);
        *subscribers = running;
        subscribers.extend(added);
    }
}

impl<T: Clone + Send + 'static> Event<T> {
    pub fn val_async<Fut, Function>(&self, action: Function)
    where
        Fut: Future + Send + 'static,
        Function: (FnMut(T) -> Fut) + Send + Clone + 'static, {
        self.val(move |val| {
            let mut a = action.clone();
            crate::deps::hreads::spawn(async move {
                a(val).await;
            });
        });
    }
}

impl<T> Default for Event<T> {
    fn default() -> Self {
        Self {
            subscribers: RefCell::default(),
            removals:    Cell::new(0),
        }
    }
}

impl<T> Debug for Event<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Event<{}>", type_name::<T>())
    }
}
