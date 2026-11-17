use thiserror::Error;

/// The camera errors an app may want to tell apart. Every other failure is
/// a plain error with its reason. Find one with `err.downcast_ref()`.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CameraError {
    #[error("the user did not allow the camera")]
    Refused,
    /// iOS and a packaged Mac app stop an app that asks for the camera with
    /// no usage text, so the engine does not ask then.
    #[error("the app has no camera usage text, set `usage` in the `[camera]` table of hilen.toml")]
    NoUsageText,
    #[error("this device has no camera support")]
    NoCamera,
    #[error("the camera {0} is not there")]
    NotFound(String),
}
