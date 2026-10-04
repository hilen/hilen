//! The system clipboard. One `set_text` on every platform, and `set_secret`
//! with `clear_if_holds` for text that must not spread. Reading back exists
//! everywhere except the browser, where reading needs an async permission
//! prompt and no app needs it yet.

#[cfg(android)]
mod android;
#[cfg(desktop)]
mod headless;
#[cfg(ios)]
mod ios;
#[cfg(macos)]
mod macos;

use anyhow::Result;
#[cfg(desktop)]
use anyhow::anyhow;
#[cfg(desktop)]
use parking_lot::Mutex;

#[cfg(desktop)]
use crate::{system::clipboard::headless::HeadlessStore, window::Window};

/// On X11 the clipboard holds data only while its owner lives, so the
/// instance stays alive for the whole process instead of per call.
#[cfg(desktop)]
static CLIPBOARD: Mutex<Option<arboard::Clipboard>> = Mutex::new(None);

/// A headless run has no display server, so a system clipboard may not
/// exist at all. This process local store stands in for it, so clipboard
/// code takes the same path on every headless machine.
#[cfg(desktop)]
static HEADLESS_CLIPBOARD: Mutex<HeadlessStore> = Mutex::new(HeadlessStore::new());

pub struct Clipboard;

impl Clipboard {
    /// Puts text on the system clipboard. The browser write lands
    /// asynchronously, so there a failure is only logged. A browser with
    /// no clipboard API gives an error.
    pub fn set_text(text: impl ToString) -> Result<()> {
        let text = text.to_string();

        #[cfg(desktop)]
        {
            if Window::headless() {
                HEADLESS_CLIPBOARD.lock().set(&text);
                return Ok(());
            }
            with_clipboard(|clipboard| Ok(clipboard.set_text(text)?))?;
            #[cfg(macos)]
            macos::note_own_write();
            Ok(())
        }

        #[cfg(wasm)]
        {
            write_in_browser(&text)
        }

        #[cfg(ios)]
        {
            ios::set_text(&text);
            Ok(())
        }

        #[cfg(android)]
        {
            android::set_text(&text, false)
        }
    }

    /// The current text content of the clipboard.
    #[cfg(not_wasm)]
    pub fn get_text() -> Result<String> {
        #[cfg(desktop)]
        {
            if Window::headless() {
                return HEADLESS_CLIPBOARD.lock().get();
            }
            with_clipboard(|clipboard| Ok(clipboard.get_text()?))
        }

        #[cfg(ios)]
        {
            ios::get_text()
        }

        #[cfg(android)]
        {
            android::get_text()
        }
    }

    /// Puts a secret on the clipboard, like a password or a recovery
    /// phrase. The copy is marked so that it stays on this device and so
    /// that clipboard managers leave it out of their history. Take it back
    /// after some seconds with [`Clipboard::clear_if_holds`].
    ///
    /// What each platform does:
    ///
    /// - macOS: the copy stays on this Mac, the universal clipboard does not
    ///   send it to other devices, and it carries the
    ///   `org.nspasteboard.ConcealedType` marker that clipboard managers honor.
    /// - iOS: the copy stays on this device. iOS has no clipboard managers and
    ///   no marker for them.
    /// - Windows: the copy stays out of the clipboard history and out of the
    ///   cloud clipboard, and clipboard managers are told to skip it.
    /// - Linux: the copy carries the `x-kde-passwordManagerHint` marker that
    ///   clipboard managers honor. Linux sends a clipboard to no other device.
    /// - Android: the clip is flagged as sensitive, so the system hides its
    ///   text in the clipboard preview. Android has no flag that keeps a clip
    ///   on the device.
    /// - Browser: a plain copy. A page can mark nothing.
    ///
    /// Every marker is a request. A clipboard manager that ignores the
    /// convention of its platform still records the text.
    pub fn set_secret(text: &str) -> Result<()> {
        #[cfg(desktop)]
        {
            if Window::headless() {
                HEADLESS_CLIPBOARD.lock().set(text);
                return Ok(());
            }

            #[cfg(macos)]
            {
                macos::set_secret(text)
            }

            #[cfg(win)]
            {
                use arboard::SetExtWindows;

                with_clipboard(|clipboard| {
                    Ok(clipboard
                        .set()
                        .exclude_from_monitoring()
                        .exclude_from_cloud()
                        .exclude_from_history()
                        .text(text)?)
                })
            }

            #[cfg(linux)]
            {
                use arboard::SetExtLinux;

                with_clipboard(|clipboard| Ok(clipboard.set().exclude_from_history().text(text)?))
            }
        }

        #[cfg(wasm)]
        {
            write_in_browser(text)
        }

        #[cfg(ios)]
        {
            ios::set_secret(text);
            Ok(())
        }

        #[cfg(android)]
        {
            android::set_text(text, true)
        }
    }

    /// Clears the clipboard when it still holds `text`, and says whether
    /// it did. Something the user copied in the meantime stays. This is
    /// how an app takes a secret back after some seconds. An empty `text`
    /// clears nothing.
    ///
    /// Where it cannot clear:
    ///
    /// - macOS and iOS: only a copy this app made is cleared. The engine
    ///   compares the change count of the pasteboard with the one after its own
    ///   last write and reads the content only when they match, since reading
    ///   what another app copied makes the system ask the user for a paste
    ///   permission.
    /// - Android: only the app in front may read the clipboard, so a call made
    ///   while the app is in the background clears nothing.
    /// - Browser: a page cannot read the clipboard without a prompt and cannot
    ///   write it without a fresh user action, so nothing is cleared and the
    ///   answer is always `false`.
    pub fn clear_if_holds(text: &str) -> Result<bool> {
        if text.is_empty() {
            return Ok(false);
        }

        #[cfg(desktop)]
        {
            if Window::headless() {
                return Ok(HEADLESS_CLIPBOARD.lock().clear_if_holds(text));
            }

            #[cfg(macos)]
            {
                Ok(macos::clear_if_holds(text))
            }

            #[cfg(not(macos))]
            {
                use arboard::Error;
                use zeroize::Zeroizing;

                with_clipboard(|clipboard| {
                    let held = match clipboard.get_text() {
                        Ok(held) => Zeroizing::new(held),
                        // An empty clipboard, or one with a picture or a
                        // file, holds no text of ours.
                        Err(Error::ContentNotAvailable) => return Ok(false),
                        Err(err) => return Err(err.into()),
                    };
                    if held.as_str() != text {
                        return Ok(false);
                    }
                    clipboard.clear()?;
                    Ok(true)
                })
            }
        }

        #[cfg(wasm)]
        {
            log::debug!("The browser lets a page clear no clipboard, the copy stays");
            Ok(false)
        }

        #[cfg(ios)]
        {
            Ok(ios::clear_if_holds(text))
        }

        #[cfg(android)]
        {
            android::clear_if_holds(text)
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

/// An old browser, a TV for one, has no `navigator.clipboard` at all and a
/// page on plain http has none either. Calling into the missing object
/// throws, so it is checked first.
#[cfg(wasm)]
fn write_in_browser(text: &str) -> Result<()> {
    let clipboard = web_sys::window().expect("Failed to get browser window").navigator().clipboard();

    if clipboard.is_undefined() || clipboard.is_null() {
        anyhow::bail!("This browser has no clipboard API");
    }

    let promise = clipboard.write_text(text);

    wasm_bindgen_futures::spawn_local(async move {
        if let Err(err) = wasm_bindgen_futures::JsFuture::from(promise).await {
            log::error!("Failed to set clipboard text: {err:?}");
        }
    });

    Ok(())
}
