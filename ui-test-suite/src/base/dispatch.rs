// use anyhow::Result;
// use hilen::{
//     dispatch::{Task, from_main, wait_for_next_frame},
//     refs::Weak,
//     hilen::ui::{Button,  Setup, Spinner, UI, ViewData,
// async_link_button, link_button, view},     ui_test::inject_touches,
// };
// use tokio::sync::{
//     Mutex,
//     oneshot::{Receiver, channel},
// };
//
// static RECEIVER: std::sync::Mutex<Option<Receiver<()>>> =
// std::sync::Mutex::new(None); static DATA: Mutex<String> =
// Mutex::const_new(String::new());
//
// #[view]
// pub struct AsyncDispatch {
//     value: u64,
//
//     #[init]
//     button:  Button,
//     button2: Button,
//     button3: Button,
// }
//
// impl AsyncDispatch {
//     fn tapped(mut self: Weak<Self>) {
//         let (se, re) = channel::<()>();
//
//         *RECEIVER.lock().unwrap() = Some(re);
//
//         let spin = Spinner::lock();
//         Task::blocking(||
// (1..100_000_000).into_iter().sum::<u64>()).callback(move |sum| {
// self.value = sum;
//
//             drop(spin);
//             se.send(()).unwrap()
//         });
//     }
//
//     async fn tapped2(self: Weak<Self>) -> Result<()> {
//         let mut st = DATA.lock().await;
//         *st = "tapped2".to_string();
//         Ok(())
//     }
//
//     async fn tapped_arg(self: Weak<Self>, arg: usize) -> Result<()> {
//         let mut st = DATA.lock().await;
//         *st = format!("tapped arg: {arg}").to_string();
//         Ok(())
//     }
// }
//
// impl Setup for AsyncDispatch {
//     fn setup(mut self: Weak<Self>) {
//         self.value = 5;
//
//         self.button.set_text("Press");
//         self.button.place().tl(20).size(100, 100);
//         link_button!(self, button, tapped);
//
//         self.button2.place().below(self.button, 20);
//         async_link_button!(self.button2, tapped2);
//
//         self.button3.place().below(self.button2, 20);
//         async_link_button!(self.button3, tapped_arg, 555);
//     }
// }
//
// pub async fn test_dispatch() -> Result<()> {
//     let mut view = UI::init_test_view::<AsyncDispatch>();
//
//     assert_eq!(view.value, 5);
//
//     from_main(move || {
//         view.value = 10;
//     })
//     .await;
//
//     assert_eq!(view.value, 10);
//
//     inject_touches(
//         "
//             94   99   b
//             94   99   e
//         ",
//     )
//     .await;
//
//     let rc = RECEIVER.lock().unwrap().take().unwrap();
//
//     assert_eq!(view.value, 10);
//
//     rc.await?;
//
//     assert_eq!(view.value, 4999999950000000);
//
//     inject_touches(
//         "
//             87   206  b
//             87   206  e
//         ",
//     )
//     .await;
//
//     assert_eq!("tapped2", DATA.lock().await.as_str());
//
//     inject_touches(
//         "
//             58   322  b
//             58   322  e
//         ",
//     )
//     .await;
////
//     assert_eq!("tapped arg: 555", DATA.lock().await.as_str());
//
//     Ok(())
// }
