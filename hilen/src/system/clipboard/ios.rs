//! The clipboard of iOS, `UIPasteboard`.

use std::time::Duration;

use anyhow::{Result, anyhow};
use objc2::{rc::Retained, runtime::AnyObject};
use objc2_foundation::{NSArray, NSDate, NSDictionary, NSNumber, NSString, ns_string};
use objc2_ui_kit::{UIPasteboard, UIPasteboardOptionExpirationDate, UIPasteboardOptionLocalOnly};
use parking_lot::Mutex;

use super::image::ClipboardImage;

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

/// The png file the system makes of the picture. A photo that is stored
/// turned comes out the way it is stored, the png has no turn mark.
pub(super) fn get_image() -> Result<Option<ClipboardImage>> {
    let Some(picture) = (unsafe { UIPasteboard::generalPasteboard().image() }) else {
        return Ok(None);
    };
    let png = picture
        .png_representation()
        .ok_or_else(|| anyhow!("The system made no png file of the picture in the clipboard"))?;

    ClipboardImage::from_png(png.to_vec()).map(Some)
}

pub(super) fn get_text() -> Result<String> {
    unsafe { UIPasteboard::generalPasteboard().string() }
        .map(|string| string.to_string())
        .ok_or_else(|| anyhow!("The clipboard holds no text"))
}

/// One plain text item with the option that keeps it on this device, so
/// the universal clipboard does not hand it to the other devices of the
/// user, and the date after which the system removes it, also while this
/// app is suspended or closed.
pub(super) fn set_secret(text: &str, stay: Duration) {
    let pasteboard = UIPasteboard::generalPasteboard();

    let text = NSString::from_str(text);
    let text: &AnyObject = &text;
    // The type `setString:` writes, so `string` reads the item back.
    let item = NSDictionary::from_slices(&[ns_string!("public.utf8-plain-text")], &[text]);
    let items = NSArray::from_slice(&[&*item]);

    unsafe { pasteboard.setItems_options(&items, &secret_options(stay)) };
    note_own_write(&pasteboard);
}

fn secret_options(stay: Duration) -> Retained<NSDictionary<NSString, AnyObject>> {
    let local_only = NSNumber::new_bool(true);
    let local_only: &AnyObject = &local_only;
    let expires = NSDate::dateWithTimeIntervalSinceNow(stay.as_secs_f64());
    let expires: &AnyObject = &expires;
    NSDictionary::from_slices(
        &[unsafe { UIPasteboardOptionLocalOnly }, unsafe {
            UIPasteboardOptionExpirationDate
        }],
        &[local_only, expires],
    )
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

/// Runs in the iOS simulator, `cargo test --target aarch64-apple-ios-sim`
/// with `xcrun simctl spawn` as the runner. A process started that way gets
/// no pasteboard, a call to it never returns, so only the options are tested.
#[cfg(test)]
mod test {
    use std::time::Duration;

    use objc2::rc::Retained;
    use objc2_foundation::{NSDate, NSNumber};
    use objc2_ui_kit::{UIPasteboardOptionExpirationDate, UIPasteboardOptionLocalOnly};

    use super::secret_options;

    #[test]
    fn a_secret_carries_its_end_and_stays_on_the_device() {
        let options = secret_options(Duration::from_secs(30));

        let local_only = options
            .objectForKey(unsafe { UIPasteboardOptionLocalOnly })
            .and_then(|value| value.downcast::<NSNumber>().ok())
            .expect("the local only option is a number");
        assert!(local_only.as_bool());

        let expires: Retained<NSDate> = options
            .objectForKey(unsafe { UIPasteboardOptionExpirationDate })
            .and_then(|value| value.downcast::<NSDate>().ok())
            .expect("the end is a date");
        let seconds = expires.timeIntervalSinceNow();
        assert!((29.0..=30.0).contains(&seconds), "{seconds}");
    }
}
