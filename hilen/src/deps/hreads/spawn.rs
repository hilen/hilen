#[cfg(not_wasm)]
use log::error;

#[cfg(wasm)]
pub fn spawn<F>(future: F)
where F: Future<Output = ()> + 'static {
    wasm_bindgen_futures::spawn_local(future);
}

#[cfg(not_wasm)]
pub fn spawn<F, O>(future: F)
where
    F: Future<Output = O> + Send + 'static,
    O: Send + 'static, {
    tokio::spawn(future);
}

#[cfg(not_wasm)]
pub fn log_spawn<O>(future: impl Future<Output = anyhow::Result<O>> + Send + 'static)
where O: Send + 'static {
    tokio::spawn(async {
        match tokio::spawn(future).await {
            Ok(exec_result) => {
                if let Err(exec_result) = exec_result {
                    error!("Future execution error: {exec_result}");
                }
            }
            Err(join_err) => {
                error!("Join error: {join_err}");
            }
        }
    });
}

/// A real worker thread that may block, unlike `spawn` which is a task
/// on the async runtime. Needs the atomics build and cross origin
/// isolation headers, and the worker starts only when the main thread
/// yields to the browser. Native code uses `std::thread::spawn` directly,
/// and only the browser test suite and inspector spawn workers.
#[cfg(all(wasm, any(feature = "ui-tests", feature = "inspect")))]
pub fn spawn_thread(work: impl FnOnce() + Send + 'static) {
    wasm_thread::spawn(work);
}

pub fn block_on<F>(future: F)
where F: Future<Output = ()> + 'static {
    #[cfg(wasm)]
    wasm_bindgen_futures::spawn_local(future);
    // pollster runs the future on the calling thread with no runtime of its
    // own, so it works from inside the tokio runtime. tokio's own block_on
    // would panic there, which is the reason async-std was used before.
    #[cfg(not_wasm)]
    pollster::block_on(future);
}

#[cfg(not_wasm)]
pub fn unasync<F, Out>(future: F) -> Out
where F: Future<Output = Out> {
    pollster::block_on(future)
}

pub async fn sleep(duration: f32) {
    #[cfg(not_wasm)]
    tokio::time::sleep(std::time::Duration::from_secs_f32(duration)).await;
    #[cfg(wasm)]
    {
        let millis = std::time::Duration::from_secs_f32(duration.max(0.0)).as_millis();
        gloo_timers::future::TimeoutFuture::new(u32::try_from(millis).unwrap_or(u32::MAX)).await;
    }
}

pub fn now() -> f64 {
    #[cfg(target_arch = "wasm32")]
    {
        // Date.now exists in every scope. performance hangs off window,
        // which a worker does not have, and the suite runs on a worker.
        web_sys::js_sys::Date::now() / 1000.0
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::time::{SystemTime, UNIX_EPOCH};

        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_secs_f64()
    }
}
