#[cfg(not(target_arch = "wasm32"))]
pub(crate) struct Config;

#[cfg(not(target_arch = "wasm32"))]
impl Config {
    pub(crate) async fn sentry_url(app: &dyn crate::App) -> Option<String> {
        match app.sentry_url().await {
            Ok(url) => url,
            Err(err) => {
                log::warn!("Failed to get sentry URL: {err}");
                None
            }
        }
    }
}
