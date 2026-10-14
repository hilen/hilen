//! A Mac app bundle gets a link of its url scheme as an Apple Event, also
//! the link that started it. `AppKit` sets its own handlers while it starts,
//! so this handler goes in when the app is about to finish its launch,
//! after those and before the Apple Events of the launch are handled. The
//! scheme itself is in the `Info.plist` of the bundle, which the packaging
//! writes.

use log::warn;
use objc2::{
    MainThreadMarker, MainThreadOnly, class, define_class, msg_send,
    rc::Retained,
    runtime::{AnyObject, NSObject},
    sel,
};
use objc2_foundation::NSString;

/// `GURL`, the class and the id of the Apple Event of a link.
const GET_URL: u32 = u32::from_be_bytes(*b"GURL");

/// `----`, the key of the direct object of an Apple Event, the link here.
const DIRECT_OBJECT: u32 = u32::from_be_bytes(*b"----");

define_class!(
    // SAFETY: NSObject has no subclassing requirements and the class has no
    // Drop.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "HilenLinkHandler"]
    struct LinkHandler;

    impl LinkHandler {
        #[unsafe(method(willFinishLaunching:))]
        fn will_finish_launching(&self, _notification: &AnyObject) {
            // SAFETY: the selector is a method of this class with the
            // arguments an Apple Event handler gets, and the manager keeps
            // the handler, which never goes away, see `install`.
            unsafe {
                let manager: Retained<AnyObject> = msg_send![class!(NSAppleEventManager), sharedAppleEventManager];
                let _: () = msg_send![
                    &manager,
                    setEventHandler: self,
                    andSelector: sel!(handleGetURLEvent:withReplyEvent:),
                    forEventClass: GET_URL,
                    andEventID: GET_URL
                ];
            }
        }

        #[unsafe(method(handleGetURLEvent:withReplyEvent:))]
        fn handle_get_url(&self, event: &AnyObject, _reply: &AnyObject) {
            // SAFETY: an `NSAppleEventDescriptor` answers both.
            let url: Option<Retained<NSString>> = unsafe {
                let descriptor: Option<Retained<AnyObject>> = msg_send![event, paramDescriptorForKeyword: DIRECT_OBJECT];
                descriptor.and_then(|descriptor| msg_send![&descriptor, stringValue])
            };
            match url {
                Some(url) => super::receive(url.to_string(), "an Apple Event"),
                None => warn!("an Apple Event for a link came with no link"),
            }
        }
    }
);

/// Runs before the event loop, on the main thread.
pub(super) fn install() {
    let Some(mtm) = MainThreadMarker::new() else {
        warn!("the link handler of macOS needs the main thread, links of the url scheme are lost");
        return;
    };
    // SAFETY: `init` of NSObject, and the observer method takes 1 object,
    // the notification.
    unsafe {
        let handler: Retained<LinkHandler> = msg_send![LinkHandler::alloc(mtm), init];
        let center: Retained<AnyObject> = msg_send![class!(NSNotificationCenter), defaultCenter];
        let name = NSString::from_str("NSApplicationWillFinishLaunchingNotification");
        let _: () = msg_send![
            &center,
            addObserver: &*handler,
            selector: sel!(willFinishLaunching:),
            name: &*name,
            object: None::<&AnyObject>
        ];
        // Neither the center nor the event manager keeps the handler, and it
        // has to answer for the whole life of the app.
        std::mem::forget(handler);
    }
}
