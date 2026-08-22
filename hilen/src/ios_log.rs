//! iOS drops stdout and stderr for a launched app, so `println!`, the fern
//! chain and the default panic hook all go nowhere. `NSLog` reaches the device
//! system log instead, which `idevicesyslog` can stream.

use std::{backtrace::Backtrace, ffi::c_void, panic};

use objc2_foundation::{NSException, NSString};

unsafe extern "C" {
    fn NSLog(format: *const NSString, ...);

    fn objc_setExceptionPreprocessor(
        handler: extern "C" fn(*mut NSException) -> *mut NSException,
    ) -> *const c_void;
}

pub(crate) fn log(message: &str) {
    // A literal format keeps any percent sign in the message from being read
    // as a format specifier.
    let format = NSString::from_str("%@");
    let message = NSString::from_str(message);
    unsafe { NSLog(&*format, &*message) };
}

/// Without this a panic leaves nothing behind but `abort()` in the crash
/// report, with no message and no location.
pub(crate) fn set_panic_hook() {
    panic::set_hook(Box::new(|info| {
        log(&format!("{info}\nBacktrace: {}", Backtrace::force_capture()));
    }));
}

/// An Objective-C exception that reaches an `extern "C"` frame, such as a
/// UIKit callback block, aborts the process as "panic in a function that
/// cannot unwind". That abort reports neither the reason nor the throw site.
/// A preprocessor runs at the throw itself, while both are still available.
extern "C" fn log_exception(exception: *mut NSException) -> *mut NSException {
    let reason = unsafe { &*exception }
        .reason()
        .map(|reason| reason.to_string())
        .unwrap_or_default();

    log(&format!(
        "Objective-C exception: {} {reason}\nBacktrace: {}",
        unsafe { &*exception }.name(),
        Backtrace::force_capture()
    ));

    exception
}

pub(crate) fn set_exception_logger() {
    unsafe { objc_setExceptionPreprocessor(log_exception) };
}
