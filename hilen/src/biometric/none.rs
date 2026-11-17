//! A system with no biometric check in the engine: Windows, Linux,
//! Android, tvOS and the browser.

use anyhow::Result;
use log::debug;

use crate::biometric::{BiometricError, BiometricKind, Done};

pub(super) fn kind() -> Result<BiometricKind> {
    Err(BiometricError::NotAvailable.into())
}

pub(super) fn check(reason: &str, done: Done) {
    debug!("no biometric check on this system, asked for: {reason}");
    done(Err(BiometricError::NotAvailable.into()));
}
