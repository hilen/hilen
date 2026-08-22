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
#[should_panic(expected = "Event already has a subscriber")]
fn double_subscriber() {
    let event = EVENT.lock().unwrap();
    event.sub(|| {});
    event.sub(|| {});
}

#[wasm_bindgen_test(unsupported = test)]
fn debug() {
    assert_eq!("Event<i32>", &format!("{:?}", Event::<i32>::default()));
}
