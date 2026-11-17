//! `LocalAuthentication` on macOS and iOS. The policy is the finger or the
//! face alone, with no way to the password of the device.

use anyhow::{Error, Result, anyhow};
use block2::RcBlock;
use objc2::{rc::Retained, runtime::Bool};
use objc2_foundation::{NSBundle, NSError, NSString, ns_string};
use objc2_local_authentication::{LABiometryType, LAContext, LAError, LAErrorDomain, LAPolicy};
use parking_lot::Mutex;

use crate::biometric::{BiometricError, BiometricKind, Done};

const POLICY: LAPolicy = LAPolicy::DeviceOwnerAuthenticationWithBiometrics;

const USAGE_KEY: &str = "NSFaceIDUsageDescription";

pub(super) fn kind() -> Result<BiometricKind> {
    ready(&context())
}

pub(super) fn check(reason: &str, done: Done) {
    let context = context();
    let kind = match ready(&context) {
        Ok(kind) => kind,
        Err(err) => return done(Err(err)),
    };
    if needs_usage_text(kind) && !has_usage_text() {
        return done(Err(BiometricError::NoUsageText.into()));
    }

    unsafe { context.setLocalizedFallbackTitle(Some(ns_string!(""))) };

    // The block is `Fn`, the answer comes once. The system cancels a check
    // whose context is released, so the block holds it until the answer.
    let pending = Mutex::new(Some((done, context.clone())));
    let block = RcBlock::new(move |passed: Bool, error: *mut NSError| {
        let Some((done, context)) = pending.lock().take() else {
            return;
        };
        done(outcome(passed.as_bool(), unsafe { error.as_ref() }));
        drop(context);
    });
    unsafe { context.evaluatePolicy_localizedReason_reply(POLICY, &NSString::from_str(reason), &block) };
}

fn context() -> Retained<LAContext> {
    unsafe { LAContext::new() }
}

/// The kind of check `context` can run now.
fn ready(context: &LAContext) -> Result<BiometricKind> {
    unsafe { context.canEvaluatePolicy_error(POLICY) }.map_err(|error| error_of(&error))?;
    kind_of(unsafe { context.biometryType() }).ok_or_else(|| BiometricError::NotAvailable.into())
}

fn kind_of(kind: LABiometryType) -> Option<BiometricKind> {
    match kind {
        LABiometryType::TouchID => Some(BiometricKind::TouchId),
        LABiometryType::FaceID => Some(BiometricKind::FaceId),
        LABiometryType::OpticID => Some(BiometricKind::OpticId),
        _ => None,
    }
}

/// Only an iPhone and an iPad ask the app for a text, and only for the face.
fn needs_usage_text(kind: BiometricKind) -> bool {
    cfg!(target_os = "ios") && kind == BiometricKind::FaceId
}

fn has_usage_text() -> bool {
    NSBundle::mainBundle()
        .objectForInfoDictionaryKey(&NSString::from_str(USAGE_KEY))
        .is_some()
}

/// The answer of the system as the answer of `Biometric::check`.
fn outcome(passed: bool, error: Option<&NSError>) -> Result<bool> {
    if passed {
        return Ok(true);
    }
    let Some(error) = error else {
        return Err(anyhow!(
            "the biometric check failed with no reason from the system"
        ));
    };
    if is_local_authentication(error) && is_refusal(LAError(error.code())) {
        return Ok(false);
    }
    Err(error_of(error))
}

fn is_refusal(code: LAError) -> bool {
    matches!(
        code,
        LAError::AuthenticationFailed
            | LAError::UserCancel
            | LAError::UserFallback
            | LAError::SystemCancel
            | LAError::AppCancel
    )
}

fn error_of(error: &NSError) -> Error {
    if is_local_authentication(error) {
        match LAError(error.code()) {
            LAError::BiometryNotAvailable => return BiometricError::NotAvailable.into(),
            LAError::BiometryNotEnrolled => return BiometricError::NotEnrolled.into(),
            LAError::BiometryLockout => return BiometricError::LockedOut.into(),
            _ => {}
        }
    }
    anyhow!(
        "{}, {} {}",
        error.localizedDescription(),
        error.domain(),
        error.code()
    )
}

fn is_local_authentication(error: &NSError) -> bool {
    error.domain().isEqualToString(unsafe { LAErrorDomain })
}

#[cfg(test)]
mod tests {
    use objc2::rc::Retained;
    use objc2_foundation::{NSError, NSString};
    use objc2_local_authentication::{LABiometryType, LAError, LAErrorDomain};

    use super::{kind_of, needs_usage_text, outcome};
    use crate::biometric::{BiometricError, BiometricKind};

    fn error(domain: &NSString, code: LAError) -> Retained<NSError> {
        unsafe { NSError::errorWithDomain_code_userInfo(domain, code.0, None) }
    }

    fn la_error(code: LAError) -> Retained<NSError> {
        error(unsafe { LAErrorDomain }, code)
    }

    #[test]
    fn a_pass_is_true() {
        assert!(outcome(true, None).unwrap());
    }

    #[test]
    fn a_cancel_or_a_failed_try_is_false() {
        for code in [
            LAError::AuthenticationFailed,
            LAError::UserCancel,
            LAError::UserFallback,
            LAError::SystemCancel,
            LAError::AppCancel,
        ] {
            assert!(!outcome(false, Some(&la_error(code))).unwrap(), "{code:?}");
        }
    }

    #[test]
    fn a_device_that_cannot_is_a_typed_error() {
        for (code, expected) in [
            (LAError::BiometryNotAvailable, BiometricError::NotAvailable),
            (LAError::BiometryNotEnrolled, BiometricError::NotEnrolled),
            (LAError::BiometryLockout, BiometricError::LockedOut),
        ] {
            let err = outcome(false, Some(&la_error(code))).unwrap_err();
            assert_eq!(err.downcast_ref::<BiometricError>(), Some(&expected));
        }
    }

    #[test]
    fn another_error_keeps_the_reason_of_the_system() {
        let err = outcome(false, Some(&la_error(LAError::PasscodeNotSet))).unwrap_err();
        assert!(err.downcast_ref::<BiometricError>().is_none());
        assert!(
            err.to_string().ends_with("com.apple.LocalAuthentication -5"),
            "{err}"
        );

        // The same code from another domain is no refusal.
        let other = error(&NSString::from_str("other.domain"), LAError::UserCancel);
        assert!(outcome(false, Some(&other)).is_err());

        assert!(outcome(false, None).is_err());
    }

    #[test]
    fn the_kinds_of_sensor() {
        assert_eq!(kind_of(LABiometryType::TouchID), Some(BiometricKind::TouchId));
        assert_eq!(kind_of(LABiometryType::FaceID), Some(BiometricKind::FaceId));
        assert_eq!(kind_of(LABiometryType::OpticID), Some(BiometricKind::OpticId));
        assert_eq!(kind_of(LABiometryType::None), None);
    }

    #[test]
    fn only_face_id_on_an_iphone_needs_the_text() {
        assert_eq!(needs_usage_text(BiometricKind::FaceId), cfg!(target_os = "ios"));
        assert!(!needs_usage_text(BiometricKind::TouchId));
    }
}
