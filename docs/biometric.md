# Biometric check

The `biometric` feature asks the user for a finger or a face before a step that needs
it, like a payment. It is off by default. The code is in `hilen/src/biometric`.

```rust
use hilen::{biometric::{Biometric, BiometricError}, dispatch::{on_main, spawn}};

// Hide the button when this is None.
let kind = Biometric::kind();

spawn(async move {
    let answer = Biometric::check("send the payment").await;
    on_main(move || match answer {
        Ok(true) => { /* passed */ }
        Ok(false) => { /* cancelled or failed, ask again or use the PIN */ }
        Err(err) => { /* the check cannot run, err.downcast_ref::<BiometricError>() */ }
    });
});
```

## The calls

- `Biometric::kind()` gives `Option<BiometricKind>`: `TouchId`, `FaceId` or
  `OpticId`. It is `Some` only when a check can run now. A device with no sensor,
  with no finger or face set up, or with the check locked after too many failed
  tries gives `None`. It answers at once, from any thread, and shows nothing.
- `Biometric::check(reason).await` gives `Result<bool>`. `Ok(true)` is a pass.
  `Ok(false)` is a cancel by the user, a failed try, or a prompt the system closed,
  like when the app went to the background. An error means the check could not run.
  `BiometricError` names the cases an app may want to tell apart: `NotAvailable`,
  `NotEnrolled`, `LockedOut`, `NoUsageText` and `NoReason`. Any other error carries
  the text, the domain and the code of the system.
- `reason` is the text of the prompt. A Mac shows it as "app is trying to reason", so
  it is a short phrase in the language of the user. An empty reason is refused with
  `NoReason` before the system sees it, the system stops the whole app on one.
- The prompt has no button for the password of the device. The policy is
  `DeviceOwnerAuthenticationWithBiometrics` and the fallback title is empty. The app
  keeps its own way in, like a PIN, for `Ok(false)` and for a lockout.
- Every call logs: `info` for the ask and the answer, `warn` for a failure, `debug`
  for the answer of `kind`.

## What each system does

```
╭───────────────────────────────┬──────────────────────────────────────────╮
│ system                        │ answer                                   │
├───────────────────────────────┼──────────────────────────────────────────┤
│ macOS                         │ Touch ID through LocalAuthentication     │
├───────────────────────────────┼──────────────────────────────────────────┤
│ iOS, iPadOS                   │ Touch ID or Face ID, the same framework  │
├───────────────────────────────┼──────────────────────────────────────────┤
│ tvOS, Windows, Linux, Android │ NotAvailable, kind is None               │
├───────────────────────────────┼──────────────────────────────────────────┤
│ the browser                   │ NotAvailable, kind is None               │
╰───────────────────────────────┴──────────────────────────────────────────╯
```

Android has no check yet. `BiometricPrompt` needs a `FragmentActivity` and the
androidx library, which the Android shell does not have.

## The Face ID text

iOS fails a Face ID check of an app whose `Info.plist` has no
`NSFaceIDUsageDescription`. The engine looks for the key first and gives `NoUsageText`
in place of the prompt. The text comes from `hilen.toml`:

```toml
[biometric]
usage = "Face ID confirms each payment you send."
```

`build/ios/build-project.py` writes it into the generated project, the same way as the
`[camera]` text, see `build/py/shared/biometric.py`. A Mac and Touch ID need no text.
The hot loader of `make device` carries a text of its own, so a hot build can ask for
Face ID too.

The iOS link needs `-framework LocalAuthentication`. A static library cannot ask for a
framework by itself, so it is in `LDFLAGS` of `build/py/shared/ios.py`, which every
`xcodebuild` of the build scripts passes.

## Tests

The unit tests in `hilen/src/biometric` need no finger. They cover the map from the
error codes of the system to the answer, the kinds of sensor, the empty reason, and a
device with no check, which answers at once.

The real prompt is an ignored test, for a person at a Mac with Touch ID:

```bash
cargo test -p hilen --features biometric --lib the_prompt_by_hand -- --ignored --nocapture
```

It prints the kind and the answer. Run it from a terminal of the user. Started from
the sandboxed shell of an agent session on a Mac with Touch ID set up, it showed no
prompt, `canEvaluatePolicy` failed with code -4, `SystemCancel`, and `kind` was `None`.
