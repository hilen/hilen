mod dispatch;
mod main_thread;
mod parallel;
mod spawn;

pub use dispatch::*;
pub use main_thread::*;
#[cfg(not_wasm)]
pub use parallel::*;
pub use spawn::*;

#[cfg(test)]
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
