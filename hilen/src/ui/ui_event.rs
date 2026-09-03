use std::any::type_name;

use parking_lot::Mutex;

use crate::deps::{hreads::assert_main_thread, netrun::Function, refs::Weak};

struct Subscriber<T: Send + Clone> {
    subscriber: Weak,
    action:     Function<T, ()>,
}

pub struct UIEvent<T: Send + Clone = ()> {
    subscribers: Mutex<Vec<Subscriber<T>>>,
}

impl<T: Send + Clone> Default for UIEvent<T> {
    fn default() -> Self {
        Self {
            subscribers: Mutex::default(),
        }
    }
}

impl<T: Send + Clone> UIEvent<T> {
    pub const fn const_new() -> Self {
        Self {
            subscribers: Mutex::new(Vec::new()),
        }
    }

    pub fn clear_subscribers(&self) -> &Self {
        self.subscribers.lock().clear();
        self
    }

    pub fn sub<U: ?Sized>(&self, subscriber: Weak<U>, mut action: impl FnMut() + Send + 'static) {
        self.val(subscriber, move |_| action());
    }

    pub fn val<U: ?Sized>(&self, subscriber: Weak<U>, action: impl FnMut(T) + Send + 'static) {
        assert!(
            subscriber.is_ok(),
            "Subscribing to a UIEvent with an invalid weak pointer: {}",
            type_name::<U>()
        );

        let mut subs = self.subscribers.lock();

        // Remove if this view is already subscribed
        subs.retain(|s| s.subscriber.raw() != subscriber.raw());

        subs.push(Subscriber {
            subscriber: subscriber.erase(),
            action:     Function::new(action),
        });
    }

    /// Whether a live subscriber is waiting for this event.
    pub(crate) fn has_subscribers(&self) -> bool {
        let mut subs = self.subscribers.lock();
        subs.retain(|s| s.subscriber.is_ok());
        !subs.is_empty()
    }

    pub(crate) fn unsubscribe<U: ?Sized>(&self, view: Weak<U>) {
        self.subscribers.lock().retain(|s| s.subscriber.raw() != view.raw());
    }

    pub fn trigger(&self, val: T)
    where T: Clone {
        assert_main_thread();

        let actions: Vec<_> = {
            let mut subs = self.subscribers.lock();
            // A subscriber may be freed by the time the event fires.
            // Calling its action would dereference a dead weak pointer.
            subs.retain(|s| s.subscriber.is_ok());
            subs.iter().map(|s| s.action.clone()).collect()
        };

        for action in actions {
            action.call(val.clone());
        }
    }
}
