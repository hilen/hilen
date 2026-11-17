//! A check of the finger or the face of the user before a step that needs
//! it, like a payment. Touch ID on a Mac, Touch ID or Face ID on an iPhone
//! and an iPad, through `LocalAuthentication`. Every other system answers
//! that the device cannot, so the same app code runs everywhere.
//!
//! ```ignore
//! use hilen::{biometric::Biometric, dispatch::{on_main, spawn}};
//!
//! if Biometric::kind().is_some() {
//!     spawn(async move {
//!         let answer = Biometric::check("send the payment").await;
//!         on_main(move || view.after_check(answer));
//!     });
//! }
//! ```
//!
//! Face ID on iOS needs the `[biometric] usage` text of `hilen.toml`, which
//! the build writes into the app. Touch ID needs no text.

#[cfg(any(target_os = "macos", target_os = "ios"))]
mod apple;
#[cfg(any(target_os = "macos", target_os = "ios"))]
use apple as system;

#[cfg(not(any(target_os = "macos", target_os = "ios")))]
mod none;
#[cfg(not(any(target_os = "macos", target_os = "ios")))]
use none as system;

#[cfg(test)]
mod biometric_test;
mod error;

use anyhow::{Result, anyhow};
use futures_channel::oneshot;
use log::{debug, info, warn};

pub use self::error::BiometricError;

/// How a system answers a check. It may call it on any thread, once.
type Done = Box<dyn FnOnce(Result<bool>) + Send>;

/// The kind of sensor a check uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum BiometricKind {
    TouchId,
    FaceId,
    /// The eyes, on an Apple Vision Pro.
    OpticId,
}

pub struct Biometric;

impl Biometric {
    /// The kind of check this device can run now. `None` when it has no
    /// sensor, when no finger or face is set up, when the check is locked
    /// after too many failed tries, and on a system with no check in the
    /// engine. An app hides its button then. Works from any thread.
    pub fn kind() -> Option<BiometricKind> {
        match system::kind() {
            Ok(kind) => {
                debug!("biometric check can run, {kind:?}");
                Some(kind)
            }
            Err(err) => {
                debug!("no biometric check: {err:#}");
                None
            }
        }
    }

    /// Asks the system for the finger or the face of the user. The system
    /// shows `reason` in its prompt, on a Mac as "app is trying to reason",
    /// so it is a short text in the language of the user. `Ok(true)` when
    /// the user passed. `Ok(false)` when the user cancelled, failed or the
    /// system closed the prompt. An error when the check cannot run, a
    /// `BiometricError` for the cases an app may want to tell apart. The
    /// prompt has no button for the password of the device, the app keeps
    /// its own way in, like a PIN.
    pub async fn check(reason: &str) -> Result<bool> {
        if reason.trim().is_empty() {
            let err = BiometricError::NoReason.into();
            warn!("biometric check not asked: {err:#}");
            return Err(err);
        }
        info!("biometric check asked: {reason}");

        let (sender, answer) = oneshot::channel();
        system::check(
            reason,
            Box::new(move |result| {
                if sender.send(result).is_err() {
                    debug!("biometric answer dropped, the caller stopped waiting");
                }
            }),
        );
        let result = answer
            .await
            .unwrap_or_else(|_| Err(anyhow!("the system ended the biometric check with no answer")));

        match &result {
            Ok(true) => info!("biometric check passed"),
            Ok(false) => info!("biometric check refused or cancelled"),
            Err(err) => warn!("biometric check failed: {err:#}"),
        }
        result
    }
}
