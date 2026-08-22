use crate::deps::hreads::{on_main, spawn};

#[cfg(not_wasm)]
pub fn from_back<F, T>(
    action: impl FnOnce() -> F + Send + 'static,
    callback: impl FnOnce(T) + Send + 'static,
) where
    T: Send + 'static,
    F: Future<Output = T> + Send,
{
    spawn(async move {
        let result = action().await;

        on_main(move || {
            callback(result);
        });
    });
}

#[cfg(wasm)]
pub fn from_back<F, T>(action: impl FnOnce() -> F + 'static, callback: impl FnOnce(T) + Send + 'static)
where
    T: Send + 'static,
    F: Future<Output = T>, {
    spawn(async move {
        let result = action().await;

        on_main(move || {
            callback(result);
        });
    });
}
