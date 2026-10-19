//! A plain cover in the clear color over the whole window while a phone app
//! is not active. The system keeps a picture of the app for its app
//! switcher, and with the cover on that picture shows nothing of the app,
//! modals included. See "The privacy cover" in `docs/dispatch.md`.

#[cfg(feature = "ui-tests")]
use std::sync::atomic::AtomicU8;
use std::sync::atomic::{AtomicBool, Ordering};

use log::{debug, info};
use plat::Platform;

use crate::{ui::damage, window::request_frame};

static ENABLED: AtomicBool = AtomicBool::new(false);
static SHOWN: AtomicBool = AtomicBool::new(false);

/// The platform rule a test runs with, see `set_test_phone`.
#[cfg(feature = "ui-tests")]
static TEST_RULE: AtomicU8 = AtomicU8::new(TEST_RULE_PLATFORM);
#[cfg(feature = "ui-tests")]
const TEST_RULE_PLATFORM: u8 = 0;
#[cfg(feature = "ui-tests")]
const TEST_RULE_DESKTOP: u8 = 1;
#[cfg(feature = "ui-tests")]
const TEST_RULE_PHONE: u8 = 2;

pub(crate) fn set_enabled(enabled: bool) {
    let was = ENABLED.swap(enabled, Ordering::Relaxed);
    if was != enabled {
        info!("Privacy cover {}", if enabled { "on" } else { "off" });
    }
    if !enabled {
        set_shown(false);
    }
}

#[cfg(feature = "ui-tests")]
pub(crate) fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

/// The cover is drawn in place of every view.
pub(crate) fn shown() -> bool {
    SHOWN.load(Ordering::Relaxed)
}

/// Called with every change of `AppActivity`, before the app hears of it.
pub(crate) fn activity_changed(active: bool) {
    set_shown(ENABLED.load(Ordering::Relaxed) && !active && phone_rule());
}

/// A desktop window that only lost the focus stays as it is. Only a phone
/// keeps a picture of the app for an app switcher.
fn phone_rule() -> bool {
    #[cfg(feature = "ui-tests")]
    match TEST_RULE.load(Ordering::Relaxed) {
        TEST_RULE_DESKTOP => return false,
        TEST_RULE_PHONE => return true,
        _ => {}
    }
    Platform::MOBILE
}

/// Makes the cover follow the rule of a phone with `Some(true)` or of a
/// desktop with `Some(false)` on any platform, so 1 test checks both.
/// `None` gives the choice back to the platform.
#[cfg(feature = "ui-tests")]
pub(crate) fn set_test_phone(phone: Option<bool>) {
    let rule = match phone {
        None => TEST_RULE_PLATFORM,
        Some(false) => TEST_RULE_DESKTOP,
        Some(true) => TEST_RULE_PHONE,
    };
    TEST_RULE.store(rule, Ordering::Relaxed);
}

fn set_shown(shown: bool) {
    if SHOWN.swap(shown, Ordering::Relaxed) == shown {
        return;
    }
    debug!("Privacy cover {}", if shown { "shown" } else { "taken off" });
    if !shown {
        // The frames under the cover drew no view, the kept pixels hold
        // only the cover.
        damage::whole_next("the privacy cover went off");
    }
    // The system text field of an edited field is a view of UIKit over
    // the frame, the frame cannot cover it.
    #[cfg(all(ios, not(tvos)))]
    // SAFETY: the main thread, every change of the activity comes from it.
    unsafe {
        crate::ui::mobile::ios::hilen_ios_text_cover(shown);
    };
    request_frame();
}
