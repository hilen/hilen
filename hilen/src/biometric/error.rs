use thiserror::Error;

/// The biometric errors an app may want to tell apart. Every other failure
/// is a plain error with the reason of the system. Find one with
/// `err.downcast_ref()`.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum BiometricError {
    #[error("this device has no biometric check")]
    NotAvailable,
    #[error("no finger or face is set up on this device")]
    NotEnrolled,
    /// The passcode or the password of the device opens the check again.
    #[error("the biometric check is locked after too many failed tries")]
    LockedOut,
    /// iOS fails a Face ID check of an app with no usage text, so the engine
    /// does not ask then.
    #[error("the app has no Face ID usage text, set `usage` in the `[biometric]` table of hilen.toml")]
    NoUsageText,
    /// The system stops the whole app on an empty reason.
    #[error("the reason of a biometric check is empty, the system shows it to the user")]
    NoReason,
}
