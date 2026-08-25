//! The system clipboard. One `set_text` on every platform. Reading back
//! exists everywhere except the browser, where reading needs an async
//! permission prompt and no app needs it yet.

use anyhow::Result;
#[cfg(any(desktop, ios))]
use anyhow::anyhow;
#[cfg(desktop)]
use parking_lot::Mutex;

#[cfg(desktop)]
use crate::window::Window;

/// On X11 the clipboard holds data only while its owner lives, so the
/// instance stays alive for the whole process instead of per call.
#[cfg(desktop)]
static CLIPBOARD: Mutex<Option<arboard::Clipboard>> = Mutex::new(None);

/// A headless run has no display server, so a system clipboard may not
/// exist at all. This process local store stands in for it, so clipboard
/// code takes the same path on every headless machine.
#[cfg(desktop)]
static HEADLESS_CLIPBOARD: Mutex<Option<String>> = Mutex::new(None);

pub struct Clipboard;

impl Clipboard {
    /// Puts text on the system clipboard. The browser write lands
    /// asynchronously, so there a failure is only logged.
    pub fn set_text(text: impl ToString) -> Result<()> {
        let text = text.to_string();

        #[cfg(desktop)]
        {
            if Window::headless() {
                *HEADLESS_CLIPBOARD.lock() = Some(text);
                return Ok(());
            }
            with_clipboard(|clipboard| Ok(clipboard.set_text(text)?))
        }

        #[cfg(wasm)]
        {
            let promise = web_sys::window()
                .expect("Failed to get browser window")
                .navigator()
                .clipboard()
                .write_text(&text);

            wasm_bindgen_futures::spawn_local(async move {
                if let Err(err) = wasm_bindgen_futures::JsFuture::from(promise).await {
                    log::error!("Failed to set clipboard text: {err:?}");
                }
            });

            Ok(())
        }

        #[cfg(ios)]
        {
            use objc2_foundation::NSString;
            use objc2_ui_kit::UIPasteboard;

            unsafe {
                UIPasteboard::generalPasteboard().setString(Some(&NSString::from_str(&text)));
            }

            Ok(())
        }

        #[cfg(android)]
        {
            android::set_text(&text)
        }
    }

    /// The current text content of the clipboard.
    #[cfg(not_wasm)]
    pub fn get_text() -> Result<String> {
        #[cfg(desktop)]
        {
            if Window::headless() {
                return HEADLESS_CLIPBOARD
                    .lock()
                    .clone()
                    .ok_or_else(|| anyhow!("The clipboard holds no text"));
            }
            with_clipboard(|clipboard| Ok(clipboard.get_text()?))
        }

        #[cfg(ios)]
        unsafe {
            use objc2_ui_kit::UIPasteboard;

            UIPasteboard::generalPasteboard()
                .string()
                .map(|string| string.to_string())
                .ok_or_else(|| anyhow!("The clipboard holds no text"))
        }

        #[cfg(android)]
        {
            android::get_text()
        }
    }
}

#[cfg(desktop)]
fn with_clipboard<T>(action: impl FnOnce(&mut arboard::Clipboard) -> Result<T>) -> Result<T> {
    let mut guard = CLIPBOARD.lock();

    if guard.is_none() {
        *guard = Some(arboard::Clipboard::new().map_err(|err| anyhow!("No clipboard available: {err}"))?);
    }

    action(guard.as_mut().expect("The clipboard was just created"))
}

#[cfg(android)]
mod android {
    use anyhow::{Result, anyhow};
    use jni::{
        jni_sig, jni_str,
        objects::{JString, JValue},
    };

    use crate::system::android_jni::with_activity;

    pub(super) fn set_text(text: &str) -> Result<()> {
        with_activity(|env, activity| {
            let service = env.new_string("clipboard")?;
            let manager = env
                .call_method(
                    activity,
                    jni_str!("getSystemService"),
                    jni_sig!("(Ljava/lang/String;)Ljava/lang/Object;"),
                    &[JValue::Object(&service)],
                )?
                .l()?;

            let label = env.new_string("hilen")?;
            let value = env.new_string(text)?;
            let clip = env
                .call_static_method(
                    jni_str!("android/content/ClipData"),
                    jni_str!("newPlainText"),
                    jni_sig!("(Ljava/lang/CharSequence;Ljava/lang/CharSequence;)Landroid/content/ClipData;"),
                    &[JValue::Object(&label), JValue::Object(&value)],
                )?
                .l()?;

            env.call_method(
                &manager,
                jni_str!("setPrimaryClip"),
                jni_sig!("(Landroid/content/ClipData;)V"),
                &[JValue::Object(&clip)],
            )?;

            Ok(())
        })
    }

    pub(super) fn get_text() -> Result<String> {
        with_activity(|env, activity| {
            let service = env.new_string("clipboard")?;
            let manager = env
                .call_method(
                    activity,
                    jni_str!("getSystemService"),
                    jni_sig!("(Ljava/lang/String;)Ljava/lang/Object;"),
                    &[JValue::Object(&service)],
                )?
                .l()?;

            let clip = env
                .call_method(
                    &manager,
                    jni_str!("getPrimaryClip"),
                    jni_sig!("()Landroid/content/ClipData;"),
                    &[],
                )?
                .l()?;

            if clip.is_null() {
                return Err(anyhow!("The clipboard holds no text"));
            }

            let item = env
                .call_method(
                    &clip,
                    jni_str!("getItemAt"),
                    jni_sig!("(I)Landroid/content/ClipData$Item;"),
                    &[JValue::Int(0)],
                )?
                .l()?;

            let text = env
                .call_method(
                    &item,
                    jni_str!("getText"),
                    jni_sig!("()Ljava/lang/CharSequence;"),
                    &[],
                )?
                .l()?;

            if text.is_null() {
                return Err(anyhow!("The clipboard holds no text"));
            }

            let string = env
                .call_method(&text, jni_str!("toString"), jni_sig!("()Ljava/lang/String;"), &[])?
                .l()?;

            let string = env.cast_local::<JString>(string)?;

            Ok(string.try_to_string(env)?)
        })
    }
}
