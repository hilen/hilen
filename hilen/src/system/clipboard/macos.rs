//! The secret calls on macOS, straight on `NSPasteboard`. `arboard` has the
//! marker for clipboard managers and no call for the option that keeps a
//! copy on this Mac.

use anyhow::{Result, bail};
use objc2_app_kit::{NSPasteboard, NSPasteboardContentsOptions, NSPasteboardTypeString};
use objc2_foundation::{NSString, ns_string};
use parking_lot::Mutex;

/// The change count of the general pasteboard right after the last write
/// of this app. While the pasteboard still has that count, it holds what
/// this app wrote.
static OWN_CHANGE: Mutex<Option<isize>> = Mutex::new(None);

/// Call right after a write to the general pasteboard that went another
/// way, through `arboard`.
pub(super) fn note_own_write() {
    *OWN_CHANGE.lock() = Some(NSPasteboard::generalPasteboard().changeCount());
}

pub(super) fn set_secret(text: &str) -> Result<()> {
    let change = write_secret(&NSPasteboard::generalPasteboard(), text)?;
    *OWN_CHANGE.lock() = Some(change);
    Ok(())
}

pub(super) fn clear_if_holds(text: &str) -> bool {
    let mut own_change = OWN_CHANGE.lock();
    let cleared = clear_own(&NSPasteboard::generalPasteboard(), *own_change, text);
    if cleared {
        *own_change = None;
    }
    cleared
}

/// Replaces the content of `pasteboard` with `text`, kept on this Mac and
/// marked for clipboard managers to skip. Returns the change count after
/// the write.
fn write_secret(pasteboard: &NSPasteboard, text: &str) -> Result<isize> {
    // Takes the place of `clearContents`, which would drop the option.
    pasteboard.prepareForNewContentsWithOptions(NSPasteboardContentsOptions::CurrentHostOnly);

    if !pasteboard.setString_forType(&NSString::from_str(text), unsafe { NSPasteboardTypeString }) {
        bail!("The pasteboard did not take the text");
    }

    // The marker of nspasteboard.org, its value is never read.
    if !pasteboard.setString_forType(ns_string!(""), ns_string!("org.nspasteboard.ConcealedType")) {
        // The text must not stay behind without its marker.
        pasteboard.clearContents();
        bail!("The pasteboard did not take the marker for clipboard managers");
    }

    Ok(pasteboard.changeCount())
}

/// Clears `pasteboard` when nothing was written to it since `own_change`
/// and it holds `text`. A moved count means another app wrote, and
/// reading that content can make macOS ask the user for a paste
/// permission, so it is left unread.
fn clear_own(pasteboard: &NSPasteboard, own_change: Option<isize>, text: &str) -> bool {
    if own_change != Some(pasteboard.changeCount()) {
        return false;
    }

    let holds = pasteboard
        .stringForType(unsafe { NSPasteboardTypeString })
        .is_some_and(|held| held.isEqualToString(&NSString::from_str(text)));

    if holds {
        pasteboard.clearContents();
    }
    holds
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use objc2::{msg_send, rc::Retained};
    use objc2_app_kit::{NSPasteboard, NSPasteboardTypeString};
    use objc2_foundation::{NSString, ns_string};

    use super::{clear_own, write_secret};

    const SECRET: &str = "twelve secret words";

    /// A pasteboard of the test alone. The general one belongs to the user
    /// and no test may write to it.
    struct Private(Retained<NSPasteboard>);

    impl Private {
        fn new() -> Self {
            Self(NSPasteboard::pasteboardWithUniqueName())
        }

        fn text(&self) -> Option<String> {
            self.0
                .stringForType(unsafe { NSPasteboardTypeString })
                .map(|text| text.to_string())
        }

        fn has_type(&self, name: &NSString) -> bool {
            self.0.types().is_some_and(|types| types.containsObject(name))
        }
    }

    impl Drop for Private {
        // A unique pasteboard lives in the pasteboard server until it is
        // released, also after this process ends.
        fn drop(&mut self) {
            unsafe { msg_send![&*self.0, releaseGlobally] }
        }
    }

    #[test]
    fn a_secret_carries_the_marker_for_clipboard_managers() -> Result<()> {
        let private = Private::new();

        let change = write_secret(&private.0, SECRET)?;

        assert_eq!(private.text().as_deref(), Some(SECRET));
        assert!(private.has_type(ns_string!("org.nspasteboard.ConcealedType")));
        assert_eq!(change, private.0.changeCount());
        Ok(())
    }

    #[test]
    fn a_secret_replaces_what_was_there_with_its_types() -> Result<()> {
        let private = Private::new();
        private.0.clearContents();
        private.0.setString_forType(ns_string!("<b>old</b>"), ns_string!("public.html"));

        write_secret(&private.0, SECRET)?;

        assert!(!private.has_type(ns_string!("public.html")));
        assert_eq!(private.text().as_deref(), Some(SECRET));
        Ok(())
    }

    #[test]
    fn a_secret_still_there_is_cleared() -> Result<()> {
        let private = Private::new();
        let change = write_secret(&private.0, SECRET)?;

        assert!(clear_own(&private.0, Some(change), SECRET));
        assert_eq!(private.text(), None);
        assert!(!private.has_type(ns_string!("org.nspasteboard.ConcealedType")));
        Ok(())
    }

    #[test]
    fn a_copy_made_since_stays() -> Result<()> {
        let private = Private::new();
        let change = write_secret(&private.0, SECRET)?;

        private.0.clearContents();
        private.0.setString_forType(ns_string!("what the user copied next"), unsafe {
            NSPasteboardTypeString
        });

        assert!(!clear_own(&private.0, Some(change), SECRET));
        assert_eq!(private.text().as_deref(), Some("what the user copied next"));
        Ok(())
    }

    #[test]
    fn the_same_text_copied_again_by_someone_else_stays() -> Result<()> {
        let private = Private::new();
        let change = write_secret(&private.0, SECRET)?;

        private.0.clearContents();
        private
            .0
            .setString_forType(&NSString::from_str(SECRET), unsafe { NSPasteboardTypeString });

        assert!(!clear_own(&private.0, Some(change), SECRET));
        assert_eq!(private.text().as_deref(), Some(SECRET));
        Ok(())
    }

    #[test]
    fn another_text_is_not_cleared() -> Result<()> {
        let private = Private::new();
        let change = write_secret(&private.0, SECRET)?;

        assert!(!clear_own(&private.0, Some(change), "some other text"));
        assert_eq!(private.text().as_deref(), Some(SECRET));
        Ok(())
    }

    #[test]
    fn nothing_is_cleared_before_this_app_wrote() {
        let private = Private::new();
        private.0.clearContents();
        private
            .0
            .setString_forType(&NSString::from_str(SECRET), unsafe { NSPasteboardTypeString });

        assert!(!clear_own(&private.0, None, SECRET));
        assert_eq!(private.text().as_deref(), Some(SECRET));
    }
}
