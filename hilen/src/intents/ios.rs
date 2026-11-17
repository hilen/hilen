//! The C functions the Swift of the intents calls, made by `hilen-mobile` from
//! the `[[intent]]` tables of `hilen.toml`. Their declarations are
//! `HilenIntents.h` in the generated project. The Swift calls them on the main
//! queue, which `UIKit` drains also while the app runs in the background with
//! no window.

use std::ffi::{CStr, CString, c_char, c_void};

use log::{debug, error, warn};
use objc2::{msg_send, runtime::AnyClass};

use crate::{deps::hreads::spawn, intents};

/// The Swift side of 1 run. It takes `ok` and the text, which lives only
/// during the call.
type Done = extern "C" fn(context: *mut c_void, ok: bool, text: *const c_char);

/// The continuation of the Swift, handed back to `Done` once.
struct Context(*mut c_void);

// SAFETY: the Swift keeps the context alive until `Done` takes it, and takes
// it from any thread.
unsafe impl Send for Context {}

fn text_of(ptr: *const c_char) -> String {
    if ptr.is_null() {
        return String::new();
    }
    // SAFETY: the Swift passes a C string that lives during the call.
    unsafe { CStr::from_ptr(ptr) }.to_string_lossy().into_owned()
}

/// A text with a 0 byte in it loses everything from that byte.
fn c_text(text: String) -> CString {
    CString::new(text).unwrap_or_else(|err| {
        let end = err.nul_position();
        let mut bytes = err.into_vec();
        bytes.truncate(end);
        CString::new(bytes).unwrap_or_default()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn hilen_intent_run(
    name: *const c_char,
    args: *const c_char,
    context: *mut c_void,
    done: Done,
) {
    let name = text_of(name);
    let answer = intents::run(&name, &text_of(args));
    let context = Context(context);
    spawn(async move {
        let context = context;
        let (ok, text) = match answer.await {
            Ok(text) => {
                debug!("intent {name} answers {text:?}");
                (true, text)
            }
            Err(err) => {
                warn!("intent {name} failed: {err:#}");
                (false, format!("{err:#}"))
            }
        };
        let text = c_text(text);
        done(context.0, ok, text.as_ptr());
    });
}

/// The values of a choice as JSON. The Swift gives the text back to
/// `hilen_intent_free`.
#[unsafe(no_mangle)]
pub extern "C" fn hilen_intent_options(parameter: *const c_char) -> *mut c_char {
    c_text(intents::options_json(&text_of(parameter))).into_raw()
}

#[unsafe(no_mangle)]
pub extern "C" fn hilen_intent_free(text: *mut c_char) {
    if text.is_null() {
        return;
    }
    // SAFETY: made by `into_raw` in `hilen_intent_options`, given back once.
    drop(unsafe { CString::from_raw(text) });
}

/// `HilenIntents.updateParameters` of the generated Swift, which is in the
/// app only when it has an `[[intent]]` table.
pub(crate) fn options_changed() {
    let Some(class) = AnyClass::get(c"HilenIntents") else {
        error!("Intents::options_changed, but the app has no HilenIntents class, see docs/intents.md");
        return;
    };
    debug!("the values of the intent choices changed");
    // SAFETY: a class method with no arguments and no result.
    unsafe {
        let _: () = msg_send![class, updateParameters];
    }
}
