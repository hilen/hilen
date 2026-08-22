// use anyhow::Result;
//
// use crate::ui::views::AlertErr;
//
// #[cfg(not_wasm)]
// pub fn on_back(task: impl Future<Output = Result<()>> + Send + 'static) {
//     std::thread::spawn(move || {
//         task.await.alert_err();
//     });
// }
