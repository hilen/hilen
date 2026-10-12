//! A page the user has to finish and come back from, a login. On an iPhone
//! it opens in a browser sheet over the app, an `SFSafariViewController`, and
//! the engine closes the sheet itself when the page has done its job. So the
//! user never leaves the app. The sheet is the system browser, Google allows
//! a login in it and refuses one in a plain web view.
//!
//! Everywhere else the page opens with `open_url` and `close` does nothing.
//! The native half is `native/ios/hilen_browser.m`.

use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::Result;

static LAST_ID: AtomicU64 = AtomicU64::new(0);

/// Opens the page and gives its id for `close`. Call it on the main thread.
///
/// `closed` runs on the main thread when the user closed the sheet by hand,
/// with Done or with a swipe. It never runs after `close`, and never on a
/// platform with no sheet.
pub(crate) fn open(url: impl ToString, closed: impl FnOnce() + Send + 'static) -> Result<u64> {
    let id = LAST_ID.fetch_add(1, Ordering::Relaxed) + 1;

    #[cfg(all(ios, not(tvos)))]
    sheet::open(id, &url.to_string(), Box::new(closed))?;

    #[cfg(not(all(ios, not(tvos))))]
    {
        drop(closed);
        crate::system::open_url(url)?;
    }

    Ok(id)
}

/// Closes the sheet of this page when it is still up. From any thread.
pub(crate) fn close(id: u64) {
    #[cfg(all(ios, not(tvos)))]
    sheet::close(id);

    #[cfg(not(all(ios, not(tvos))))]
    let _ = id;
}

#[cfg(all(ios, not(tvos)))]
mod sheet {
    use std::ffi::{CString, c_char};

    use anyhow::{Result, bail};
    use parking_lot::Mutex;

    use crate::deps::hreads::{is_main_thread, on_main};

    type Closed = Box<dyn FnOnce() + Send>;

    /// The sheet that is up, with what its owner wants to hear when the
    /// user closes it.
    static SHOWN: Mutex<Option<(u64, Closed)>> = Mutex::new(None);

    unsafe extern "C" {
        fn hilen_browser_open(url: *const c_char, closed: extern "C" fn()) -> bool;
        fn hilen_browser_close();
    }

    pub(super) fn open(id: u64, url: &str, closed: Closed) -> Result<()> {
        if !is_main_thread() {
            bail!("Failed to open {url}: not on the main thread");
        }
        let address = CString::new(url)?;

        // The native side closes a sheet that is still up and tells nobody,
        // so its owner must not hear of a close either.
        drop(SHOWN.lock().take());

        // SAFETY: the string lives for the call, and this is the main thread.
        if !unsafe { hilen_browser_open(address.as_ptr(), closed_by_user) } {
            bail!("Failed to open {url} in a browser sheet");
        }
        *SHOWN.lock() = Some((id, closed));
        log::debug!("browser sheet {id} opened");
        Ok(())
    }

    pub(super) fn close(id: u64) {
        on_main(move || {
            // Another page can have the sheet by now, that one stays.
            if SHOWN.lock().take_if(|(shown, _)| *shown == id).is_none() {
                return;
            }
            // SAFETY: `on_main` runs this on the main thread.
            unsafe { hilen_browser_close() };
            log::debug!("browser sheet {id} closed by the app");
        });
    }

    extern "C" fn closed_by_user() {
        let shown = SHOWN.lock().take();
        if let Some((id, closed)) = shown {
            log::debug!("browser sheet {id} closed by the user");
            closed();
        }
    }
}
