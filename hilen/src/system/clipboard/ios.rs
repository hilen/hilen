//! The clipboard of iOS, `UIPasteboard`.

use anyhow::{Result, anyhow};
use objc2::runtime::AnyObject;
use objc2_foundation::{NSArray, NSDictionary, NSNumber, NSString, ns_string};
use objc2_ui_kit::{UIPasteboard, UIPasteboardOptionLocalOnly};
use parking_lot::Mutex;

/// The change count of the general pasteboard right after the last write
/// of this app. While the pasteboard still has that count, it holds what
/// this app wrote.
static OWN_CHANGE: Mutex<Option<isize>> = Mutex::new(None);

fn note_own_write(pasteboard: &UIPasteboard) {
    *OWN_CHANGE.lock() = Some(unsafe { pasteboard.changeCount() });
}

pub(super) fn set_text(text: &str) {
    let pasteboard = UIPasteboard::generalPasteboard();
    unsafe { pasteboard.setString(Some(&NSString::from_str(text))) };
    note_own_write(&pasteboard);
}

pub(super) fn get_text() -> Result<String> {
    unsafe { UIPasteboard::generalPasteboard().string() }
        .map(|string| string.to_string())
        .ok_or_else(|| anyhow!("The clipboard holds no text"))
}

/// One plain text item with the option that keeps it on this device, so
/// the universal clipboard does not hand it to the other devices of the
/// user.
pub(super) fn set_secret(text: &str) {
    let pasteboard = UIPasteboard::generalPasteboard();

    let text = NSString::from_str(text);
    let text: &AnyObject = &text;
    // The type `setString:` writes, so `string` reads the item back.
    let item = NSDictionary::from_slices(&[ns_string!("public.utf8-plain-text")], &[text]);
    let items = NSArray::from_slice(&[&*item]);

    let local_only = NSNumber::new_bool(true);
    let local_only: &AnyObject = &local_only;
    let options = NSDictionary::from_slices(&[unsafe { UIPasteboardOptionLocalOnly }], &[local_only]);

    unsafe { pasteboard.setItems_options(&items, &options) };
    note_own_write(&pasteboard);
}

/// Clears the pasteboard when nothing was written to it since the last
/// write of this app and it holds `text`. A moved count means another app
/// wrote, and reading that content makes iOS ask the user to allow the
/// paste, so it is left unread.
pub(super) fn clear_if_holds(text: &str) -> bool {
    let pasteboard = UIPasteboard::generalPasteboard();
    let mut own_change = OWN_CHANGE.lock();

    if *own_change != Some(unsafe { pasteboard.changeCount() }) {
        return false;
    }

    let holds =
        unsafe { pasteboard.string() }.is_some_and(|held| held.isEqualToString(&NSString::from_str(text)));

    if holds {
        unsafe { pasteboard.setItems(&NSArray::new()) };
        *own_change = None;
    }
    holds
}
