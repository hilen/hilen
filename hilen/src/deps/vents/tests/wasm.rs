#![cfg(test)]

use std::sync::{Arc, Mutex};

use wasm_bindgen_test::wasm_bindgen_test;

use crate::deps::vents::Event;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test(unsupported = test)]
fn event() {
    let event = Event::<u32>::default();

    let summ = Arc::new(Mutex::new(0));

    let summ_2 = summ.clone();
    event.val(move |val| {
        *summ_2.lock().unwrap() += val;
    });

    assert_eq!(*summ.lock().unwrap(), 0);
    event.trigger(20);
    assert_eq!(*summ.lock().unwrap(), 20);
    event.trigger(20);
    assert_eq!(*summ.lock().unwrap(), 40);

    event.remove_subscribers();
    event.trigger(20);
    assert_eq!(*summ.lock().unwrap(), 40);
}

#[wasm_bindgen_test(unsupported = test)]
fn event_async_val() {
    let event = Event::<u32>::default();

    event.val_async(async move |val| {
        dbg!(&val);
    });
}

static EVENT: Mutex<Event<()>> = Mutex::new(Event::const_default());

#[wasm_bindgen_test(unsupported = test)]
fn every_subscriber_is_called() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let event = EVENT.lock().unwrap();
    for id in 0..3 {
        let calls = calls.clone();
        event.sub(move || calls.lock().unwrap().push(id));
    }
    event.trigger(());
    assert_eq!(*calls.lock().unwrap(), vec![0, 1, 2]);
}

// Views capture a `Weak` of themselves, which is `Send`. The test stands in
// with a pointer to an event that outlives every callback.
#[derive(Clone, Copy)]
struct Shared(*const Event<u32>);
unsafe impl Send for Shared {}

impl Shared {
    fn get(self) -> &'static Event<u32> {
        unsafe { &*self.0 }
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn subscribe_and_remove_inside_a_callback() {
    let event: &'static Event<u32> = Box::leak(Box::default());
    let shared = Shared(event);
    let seen = Arc::new(Mutex::new(Vec::new()));

    let inner_seen = seen.clone();
    event.val(move |_| {
        if inner_seen.lock().unwrap().is_empty() {
            let late_seen = inner_seen.clone();
            shared.get().val(move |val| late_seen.lock().unwrap().push(val * 10));
        }
    });
    event.trigger(1);
    assert!(seen.lock().unwrap().is_empty());
    event.trigger(2);
    assert_eq!(*seen.lock().unwrap(), vec![20]);

    event.sub(move || shared.get().remove_subscribers());
    event.trigger(3);
    assert!(!event.has_subscribers());
}

#[wasm_bindgen_test(unsupported = test)]
fn debug() {
    assert_eq!("Event<i32>", &format!("{:?}", Event::<i32>::default()));
}
