//! The system clipboard. One `set_text` on every platform, and `set_secret`
//! with `clear_if_holds` for text that must not spread. Reading back exists
//! everywhere except the browser, where reading needs an async permission
//! prompt and no app needs it yet. A picture is read with `get_image`, see
//! there for what each platform does.

#[cfg(android)]
mod android;
#[cfg(desktop)]
mod headless;
mod image;
#[cfg(ios)]
mod ios;
#[cfg(macos)]
mod macos;

#[cfg(desktop)]
use std::{
    borrow::Cow,
    sync::atomic::{AtomicBool, Ordering},
};

use anyhow::Result;
#[cfg(desktop)]
use anyhow::anyhow;
#[cfg(not(desktop))]
use anyhow::bail;
#[cfg(desktop)]
use arboard::{Error, ImageData};
pub use image::ClipboardImage;
#[cfg(desktop)]
use parking_lot::Mutex;

#[cfg(desktop)]
use crate::{
    system::clipboard::{headless::HeadlessStore, image::Pixels},
    window::Window,
};

/// On X11 the clipboard holds data only while its owner lives, so the
/// instance stays alive for the whole process instead of per call.
#[cfg(desktop)]
static CLIPBOARD: Mutex<Option<arboard::Clipboard>> = Mutex::new(None);

/// A headless run has no display server, so a system clipboard may not
/// exist at all. This process local store stands in for it, so clipboard
/// code takes the same path on every headless machine.
#[cfg(desktop)]
static HEADLESS_CLIPBOARD: Mutex<HeadlessStore> = Mutex::new(HeadlessStore::new());

/// Set by `Clipboard::set_in_process`.
#[cfg(desktop)]
static IN_PROCESS: AtomicBool = AtomicBool::new(false);

/// The clipboard calls go to the store of this process, not to the system.
#[cfg(desktop)]
fn in_process() -> bool {
    Window::headless() || IN_PROCESS.load(Ordering::Relaxed)
}

pub struct Clipboard;

impl Clipboard {
    /// Puts text on the system clipboard. The browser write lands
    /// asynchronously, so there a failure is only logged. A browser with
    /// no clipboard API gives an error.
    pub fn set_text(text: impl ToString) -> Result<()> {
        let text = text.to_string();

        #[cfg(desktop)]
        {
            if in_process() {
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
            if in_process() {
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

    /// The picture in the clipboard as a png file, `None` when the
    /// clipboard holds no picture.
    ///
    /// A big picture takes time, the system hands over raw pixels and the
    /// engine makes the png file: 50 to 500 ms for 3000 by 2000 pixels. So
    /// call it off the main thread and go back with `on_main`. `TextField`
    /// does that for a paste, see its `image_pasted`.
    ///
    /// What each platform does:
    ///
    /// - macOS, Windows and Linux: the picture a screenshot or a copy in an
    ///   image editor or a browser left. On Linux only under X11 or `XWayland`.
    /// - iOS: the picture of the pasteboard, as the png file the system makes
    ///   of it. iOS can ask the user to allow the paste.
    /// - Android: always `None`. A picture is there a link to the app that
    ///   holds it, and the engine does not read such a link yet.
    /// - Browser: always `None`. A page reads a picture only through a call
    ///   that answers later, after a permission prompt.
    pub fn get_image() -> Result<Option<ClipboardImage>> {
        #[cfg(desktop)]
        {
            // The lock is given back before the encoding, which is the
            // slow part.
            let held = if in_process() {
                HEADLESS_CLIPBOARD.lock().picture()
            } else {
                system_picture()?
            };
            held.map(|picture| ClipboardImage::encode(&picture)).transpose()
        }

        #[cfg(ios)]
        {
            ios::get_image()
        }

        #[cfg(any(android, wasm))]
        {
            Ok(None)
        }
    }

    /// Puts a picture on the clipboard, in the place of what was there.
    /// Desktop only, every other platform gives an error.
    pub fn set_image(image: &ClipboardImage) -> Result<()> {
        #[cfg(desktop)]
        {
            let picture = image.decode()?;

            if in_process() {
                HEADLESS_CLIPBOARD.lock().set_picture(picture);
                return Ok(());
            }

            let data = ImageData {
                width:  picture.width.try_into()?,
                height: picture.height.try_into()?,
                bytes:  Cow::Owned(picture.rgba),
            };
            with_clipboard(|clipboard| Ok(clipboard.set_image(data)?))?;
            #[cfg(macos)]
            macos::note_own_write();
            Ok(())
        }

        #[cfg(not(desktop))]
        {
            bail!(
                "No picture of {} by {} pixels goes to the clipboard on this platform",
                image.width,
                image.height
            );
        }
    }

    /// Keeps every clipboard call inside this process, the way a headless
    /// run does, so nothing reads or replaces what the user copied. For a
    /// UI test that runs in a window. The test runner turns it off again
    /// before every test.
    #[cfg(desktop)]
    pub fn set_in_process(in_process: bool) {
        IN_PROCESS.store(in_process, Ordering::Relaxed);
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
            if in_process() {
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
            if in_process() {
                return Ok(HEADLESS_CLIPBOARD.lock().clear_if_holds(text));
            }

            #[cfg(macos)]
            {
                Ok(macos::clear_if_holds(text))
            }

            #[cfg(not(macos))]
            {
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

/// The picture of the system clipboard as the pixels `arboard` gives.
#[cfg(desktop)]
fn system_picture() -> Result<Option<Pixels>> {
    let data = match with_clipboard(|clipboard| Ok(clipboard.get_image()))? {
        Ok(data) => data,
        // An empty clipboard, or one with a text or a file.
        Err(Error::ContentNotAvailable) => return Ok(None),
        Err(err) => return Err(err.into()),
    };

    Ok(Some(Pixels {
        width:  data.width.try_into()?,
        height: data.height.try_into()?,
        rgba:   data.bytes.into_owned(),
    }))
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
