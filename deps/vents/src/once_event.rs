use std::{
    any::type_name,
    cell::RefCell,
    fmt::{Debug, Formatter},
    sync::mpsc::{Receiver, Sender, channel},
};

use log::error;

type Callback<T> = Box<dyn FnOnce(T) + Send + 'static>;

pub struct OnceEvent<T = ()> {
    once_subscriber: RefCell<Option<Callback<T>>>,
    once_sender:     RefCell<Option<Sender<T>>>,
}

impl<T: 'static> OnceEvent<T> {
    pub const fn const_default() -> Self {
        Self {
            once_subscriber: RefCell::new(None),
            once_sender:     RefCell::new(None),
        }
    }

    fn check_empty(&self) {
        assert!(
            self.once_sender.borrow().is_none(),
            "Event already has once_sender"
        );
        assert!(
            self.once_subscriber.borrow().is_none(),
            "Event already has once_subscriber"
        );
    }

    pub fn sub(&self, action: impl FnOnce() + Send + 'static) {
        self.check_empty();
        self.once_subscriber.replace(Some(Box::new(|_| action())));
    }

    pub fn val(&self, action: impl FnOnce(T) + Send + 'static) {
        self.check_empty();
        self.once_subscriber.replace(Some(Box::new(action)));
    }

    pub fn receiver(&self) -> Receiver<T> {
        self.check_empty();
        let (s, r) = channel();
        self.once_sender.replace(s.into());
        r
    }

    pub fn trigger(&self, value: T) {
        if let Some(sub) = self.once_subscriber.borrow_mut().take() {
            (sub)(value);
        } else if let Some(send) = self.once_sender.borrow_mut().take()
            && send.send(value).is_err()
        {
            error!("Failed to once send OnceEvent of type: {}", type_name::<T>());
        }
    }

    pub fn remove_subscribers(&self) {
        self.once_subscriber.replace(None);
        self.once_sender.replace(None);
    }
}

// impl<T: 'static + Send> IntoFuture for &OnceEvent<T> {
//     type Output = Result<T, RecvError>;
//     type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send>>;
//
//     fn into_future(self) -> Self::IntoFuture {
//         let recv = self.val_async();
//         Box::pin(recv)
//     }
// }

impl<T> Default for OnceEvent<T> {
    fn default() -> Self {
        Self {
            once_subscriber: RefCell::default(),
            once_sender:     RefCell::default(),
        }
    }
}

impl<T> Debug for OnceEvent<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "OnceEvent<{}>", type_name::<T>())
    }
}

#[cfg(test)]
mod test {

    use std::sync::{Arc, Mutex};

    use wasm_bindgen_test::wasm_bindgen_test;

    use crate::OnceEvent;

    #[wasm_bindgen_test(unsupported = test)]
    fn event_once() {
        let event = OnceEvent::<u32>::default();
        let summ = Arc::new(Mutex::new(0));

        let check = summ.clone();

        let sum_2 = summ.clone();
        event.val(move |val| {
            *sum_2.lock().unwrap() += val;
        });

        assert_eq!(*check.lock().unwrap(), 0);
        event.trigger(20);
        assert_eq!(*check.lock().unwrap(), 20);
        event.trigger(20);
        assert_eq!(*check.lock().unwrap(), 20);

        event.val(move |val| {
            *summ.lock().unwrap() += val;
        });

        event.remove_subscribers();

        event.trigger(20);
        assert_eq!(*check.lock().unwrap(), 20);
    }

    // #[test]
    // fn event_once_await() {
    //     let event = OnceEvent::<u32>::default();
    //     let summ = Arc::new(Mutex::new(0));
    //
    //     let res_summ = summ.clone();
    //     let join = spawn(move || {
    //         assert_eq!(summ.lock().unwrap().deref(), &0);
    //
    //         let val = event.val_async().recv().unwrap();
    //
    //         assert_eq!(val, 10);
    //
    //         *summ.lock().unwrap() += val;
    //     });
    //
    //     event.trigger(10);
    //
    //     join.join().unwrap();
    //
    //     assert_eq!(*res_summ.lock().unwrap(), 10);
    // }

    static EVENT: Mutex<OnceEvent<()>> = Mutex::new(OnceEvent::const_default());

    #[wasm_bindgen_test(unsupported = test)]
    #[should_panic(expected = "Event already has once_subscriber")]
    fn double_subscriber() {
        let event = EVENT.lock().unwrap();
        event.sub(|| {});
        event.val(|()| {});
    }

    #[wasm_bindgen_test(unsupported = test)]
    fn debug() {
        assert_eq!("OnceEvent<i32>", &format!("{:?}", OnceEvent::<i32>::default()));
    }
}
