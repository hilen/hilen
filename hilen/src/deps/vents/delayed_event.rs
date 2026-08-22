// use std::{
//     any::type_name,
//     fmt::{Debug, Formatter},
//     sync::{Arc, Mutex},
// };
//
// use chrono::Utc;
//
// use crate::deps::vents::threads::{sleep, spawn};
//
// struct Vent<T> {
//     subscriber: Option<Box<dyn FnMut(T) + Send + 'static>>,
//     delay:      f32,
//     queue:      Vec<i64>,
//     dropped:    bool,
// }
//
// pub struct DelayedEvent<T = ()> {
//     vent: Arc<Mutex<Vent<T>>>,
// }
//
// impl<T: 'static> DelayedEvent<T> {
//     pub fn set_delay(&self, delay: f32) {
//         self.vent.lock().unwrap().delay = delay;
//     }
//
//     pub fn with_delay(self, delay: f32) -> Self {
//         self.vent.lock().unwrap().delay = delay;
//         self
//     }
//
//     pub fn val(&self, action: impl FnMut(T) + Send + 'static) {
//         let mut vent = self.vent.lock().unwrap();
//         if vent.subscriber.is_some() {
//             drop(vent);
//             panic!("Event already has a subscriber");
//         }
//         vent.subscriber = Some(Box::new(action));
//     }
//
//     pub fn trigger(&self, value: T)
//     where T: Send + Debug {
//         let mut vent = self.vent.lock().unwrap();
//
//         if vent.subscriber.is_none() {
//             return;
//         }
//
//         let delay = vent.delay;
//
//         if delay == 0.0 {
//             if let Some(sub) = vent.subscriber.as_mut() {
//                 sub(value);
//                 return;
//             }
//         }
//
//         let timestamp = Utc::now().timestamp_micros();
//         vent.queue.push(timestamp);
//
//         drop(vent);
//
//         let vent = self.vent.clone();
//
//         spawn(spawn_delay(delay, vent, timestamp, value));
//     }
//
//     pub fn remove_subscribers(&self) {
//         self.vent.lock().unwrap().subscriber = None;
//     }
// }
//
// async fn spawn_delay<T>(delay: f32, vent: Arc<Mutex<Vent<T>>>, timestamp:
// i64, value: T) {     sleep((delay * 1000.0) as u32).await;
//
//     let mut vent = vent.lock().unwrap();
//
//     if vent.dropped {
//         return;
//     }
//
//     if vent.queue.is_empty() {
//         return;
//     }
//
//     if vent.queue.last().unwrap() != &timestamp {
//         return;
//     }
//
//     if let Some(sub) = vent.subscriber.as_mut() {
//         sub(value);
//     }
//
//     vent.queue.clear();
// }
//
// impl<T> Default for DelayedEvent<T> {
//     fn default() -> Self {
//         Self {
//             vent: Arc::new(Mutex::new(Vent {
//                 subscriber: None,
//                 delay:      0.0,
//                 queue:      vec![],
//                 dropped:    false,
//             })),
//         }
//     }
// }
//
// impl<T> Drop for DelayedEvent<T> {
//     fn drop(&mut self) {
//         self.vent.lock().unwrap().dropped = true;
//     }
// }
//
// impl<T> Debug for DelayedEvent<T> {
//     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//         write!(f, "DelayedEvent<{}>", type_name::<T>(),)
//     }
// }
//
// #[cfg(test)]
// mod test {
//     use std::{
//         ops::Deref,
//         sync::{Arc, Mutex},
//     };
//
//     use wasm_bindgen_test::wasm_bindgen_test;
//
//     use crate::deps::vents::{delayed_event::DelayedEvent,
// threads::busy_sleep};     // #[wasm_bindgen_test(unsupported = test)]
//     // fn delayed_event() {
//     //     let event = DelayedEvent::<i32>::default().with_delay(0.25);
//     //
//     //     let data: Arc<Mutex<Vec<i32>>> = Arc::new(Mutex::new(vec![]));
//     //
//     //     let data_clone = data.clone();
//     //     event.val(move |value| {
//     //         data_clone.lock().unwrap().push(value);
//     //     });
//     //
//     //     event.trigger(10);
//     //
//     //     sleep(Duration::from_millis(260));
//     //
//     //     for _ in 0..100 {
//     //         event.trigger(20);
//     //     }
//     //
//     //     sleep(Duration::from_millis(100));
//     //
//     //     for _ in 0..100 {
//     //         event.trigger(30);
//     //         event.trigger(31);
//     //         event.trigger(32);
//     //         event.trigger(33);
//     //         event.trigger(34);
//     //         event.trigger(35);
//     //         event.trigger(36);
//     //     }
//     //
//     //     sleep(Duration::from_millis(260));
//     //
//     //     event.trigger(40);
//     //
//     //     sleep(Duration::from_millis(260));
//     //
//     //     event.trigger(50);
//     //     event.trigger(60);
//     //
//     //     sleep(Duration::from_millis(260));
//     //
//     //     event.trigger(70);
//     //     event.trigger(90);
//     //
//     //     drop(event);
//     //
//     //     sleep(Duration::from_millis(260));
//     //
//     //     // TODO: Fix this test
//     //
//     //     // assert_eq!(data.lock().unwrap().deref(), &vec![10, 36, 40,
// 60]);     // }
//
//     // #[wasm_bindgen_test(unsupported = test)]
//     // #[should_panic]
//     // fn double_subscriber() {
//     //     let event: DelayedEvent = DelayedEvent::default();
//     //     event.val(|_| {});
//     //     event.val(|_| {});
//     // }
//
//     #[wasm_bindgen_test(unsupported = test)]
//     fn remove_subscriber() {
//         #[cfg(target_arch = "wasm32")]
//         {
//             // Sets up panics to go to the console.error in browser
// environments
// std::panic::set_hook(Box::new(console_error_panic_hook::hook));
// console_log::init_with_level(log::Level::Warn).expect("Couldn't initialize
// logger");
//
//             log::info!("Hello from wasm");
//         }
//
//         let event = DelayedEvent::<i32>::default();
//
//         event.set_delay(0.1);
//
//         let data: Arc<Mutex<Vec<i32>>> = Arc::new(Mutex::new(vec![]));
//
//         let data_clone = data.clone();
//         event.val(move |value| {
//             data_clone.lock().unwrap().push(value);
//         });
//
//         event.trigger(10);
//
//         busy_sleep(50);
//
//         event.trigger(20);
//
//         busy_sleep(110);
//
//         event.remove_subscribers();
//
//         event.trigger(30);
//
//         busy_sleep(110);
//
//         panic!("{:?}", data.lock().unwrap().deref());
//
//         // assert_eq!(data.lock().unwrap().deref(), &vec![20]);
//     }
//
//     #[wasm_bindgen_test(unsupported = test)]
//     fn debug() {
//         assert_eq!(
//             "DelayedEvent<i32>",
//             &format!("{:?}", DelayedEvent::<i32>::default())
//         );
//     }
// }
