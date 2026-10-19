use crate::biometric::{Biometric, BiometricError};

#[tokio::test]
async fn an_empty_reason_is_refused_before_the_system() {
    let err = Biometric::check(" ").await.unwrap_err();
    assert_eq!(
        err.downcast_ref::<BiometricError>(),
        Some(&BiometricError::NoReason)
    );
}

#[tokio::test]
async fn a_device_with_no_check_answers_at_once() {
    if Biometric::kind().is_some() {
        // This machine would show the prompt, the test below is for it.
        return;
    }
    let err = Biometric::check("test the engine").await.unwrap_err();
    assert!(
        matches!(
            err.downcast_ref::<BiometricError>(),
            Some(BiometricError::NotAvailable | BiometricError::NotEnrolled | BiometricError::LockedOut)
        ),
        "{err:#}"
    );
}

#[cfg(not(any(target_os = "macos", target_os = "ios")))]
#[test]
fn a_system_with_no_check_says_so() {
    assert_eq!(Biometric::kind(), None);
}

/// The real prompt, for a person at the Mac:
/// `cargo test -p hilen --features biometric --lib the_prompt_by_hand --
/// --ignored --nocapture`.
#[tokio::test]
#[ignore = "needs a finger on the sensor"]
async fn the_prompt_by_hand() {
    println!("kind: {:?}", Biometric::kind());
    println!(
        "answer: {:?}",
        Biometric::check("test the biometric check of hilen").await
    );
}
