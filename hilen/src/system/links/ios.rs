//! An iPhone opens the app for a universal link, an https link of a domain
//! that `[links]` of `hilen.toml` names, and for a link of the url scheme.
//! `hilen-mobile` writes the associated domains entitlement and the scheme.
//! iOS gives such a link to the delegate of the app, as
//! `application:continueUserActivity:restorationHandler:` and
//! `application:openURL:options:`.
//!
//! winit starts `UIApplicationMain` with no delegate, so the engine sets its
//! own when the launch is over, in the notification of it. The link of a
//! cold start is in that notification too. iOS may give the same link to the
//! delegate again after it, that second copy is dropped.

use log::{debug, warn};
use objc2::{
    MainThreadMarker, MainThreadOnly, class, define_class, msg_send,
    rc::Retained,
    runtime::{AnyObject, Bool, NSObject},
    sel,
};
use objc2_foundation::{NSString, NSURL};

use crate::deps::refs::main_lock::MainLock;

/// The link of a cold start, taken from the launch notification, until the
/// delegate gets its copy.
static LAUNCH_LINK: MainLock<Option<String>> = MainLock::new();

const BROWSING_WEB: &str = "NSUserActivityTypeBrowsingWeb";

define_class!(
    // SAFETY: NSObject has no subclassing requirements and the class has no
    // Drop.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "HilenAppDelegate"]
    struct AppDelegate;

    impl AppDelegate {
        #[unsafe(method(didFinishLaunching:))]
        fn did_finish_launching(&self, notification: &AnyObject) {
            // SAFETY: the shared application exists once it has launched.
            let app: Retained<AnyObject> = unsafe { msg_send![class!(UIApplication), sharedApplication] };
            let current: Option<Retained<AnyObject>> = unsafe { msg_send![&app, delegate] };
            if current.is_some() {
                warn!("the app has a delegate already, it gets no link");
                return;
            }
            // SAFETY: the delegate property does not keep the object, and
            // this one is never released, see `install`.
            unsafe {
                let _: () = msg_send![&app, setDelegate: self];
            }

            if let Some(url) = launch_link(notification) {
                *LAUNCH_LINK.get_mut() = Some(url.clone());
                super::receive(url, "the launch of iOS");
            }
        }

        #[unsafe(method(application:openURL:options:))]
        fn open_url(&self, _app: &AnyObject, url: &NSURL, _options: &AnyObject) -> Bool {
            match url.absoluteString() {
                Some(url) => take(url.to_string(), "openURL of iOS"),
                None => warn!("iOS opened the app for a link with no text"),
            }
            Bool::YES
        }

        #[unsafe(method(application:continueUserActivity:restorationHandler:))]
        fn continue_activity(&self, _app: &AnyObject, activity: &AnyObject, _restore: *mut AnyObject) -> Bool {
            match activity_link(activity) {
                Some(url) => {
                    take(url, "a universal link of iOS");
                    Bool::YES
                }
                None => Bool::NO,
            }
        }
    }
);

/// A link from the delegate, unless it is the one the launch gave already.
fn take(url: String, source: &str) {
    let launch = LAUNCH_LINK.get_mut().take();
    if launch.as_deref() == Some(url.as_str()) {
        debug!("{source} gave the link of the launch again");
        return;
    }
    super::receive(url, source);
}

/// The web page link of a user activity of the browsing kind.
fn activity_link(activity: &AnyObject) -> Option<String> {
    // SAFETY: an `NSUserActivity` answers both.
    let kind: Retained<NSString> = unsafe { msg_send![activity, activityType] };
    if kind.to_string() != BROWSING_WEB {
        debug!("a user activity of another kind, {kind}");
        return None;
    }
    let url: Option<Retained<NSURL>> = unsafe { msg_send![activity, webpageURL] };
    Some(url?.absoluteString()?.to_string())
}

/// The link in the launch options of a cold start: a url of the scheme or
/// the user activity of a universal link.
fn launch_link(notification: &AnyObject) -> Option<String> {
    // SAFETY: the launch notification carries the launch options as its
    // user info, a dictionary or nil.
    unsafe {
        let options: Option<Retained<AnyObject>> = msg_send![notification, userInfo];
        let options = options?;
        let key = NSString::from_str("UIApplicationLaunchOptionsURLKey");
        let url: Option<Retained<NSURL>> = msg_send![&options, objectForKey: &*key];
        if let Some(url) = url {
            return Some(url.absoluteString()?.to_string());
        }
        let key = NSString::from_str("UIApplicationLaunchOptionsUserActivityDictionaryKey");
        let activities: Option<Retained<AnyObject>> = msg_send![&options, objectForKey: &*key];
        let activities = activities?;
        let key = NSString::from_str("UIApplicationLaunchOptionsUserActivityKey");
        let activity: Option<Retained<AnyObject>> = msg_send![&activities, objectForKey: &*key];
        activity_link(&*activity?)
    }
}

/// Runs before the event loop starts `UIApplicationMain`, on the main
/// thread. A hot build runs in a loader app that launched long ago and has
/// its own delegate, it gets no link.
pub(crate) fn install() {
    let Some(mtm) = MainThreadMarker::new() else {
        warn!("the app delegate needs the main thread, the app gets no link");
        return;
    };
    // SAFETY: nil before `UIApplicationMain`.
    let running: Option<Retained<AnyObject>> = unsafe { msg_send![class!(UIApplication), sharedApplication] };
    if running.is_some() {
        debug!("the app runs inside another one, it gets no link");
        return;
    }
    // SAFETY: `init` of NSObject, and the observer method takes 1 object,
    // the notification.
    unsafe {
        let delegate: Retained<AppDelegate> = msg_send![AppDelegate::alloc(mtm), init];
        let center: Retained<AnyObject> = msg_send![class!(NSNotificationCenter), defaultCenter];
        let name = NSString::from_str("UIApplicationDidFinishLaunchingNotification");
        let _: () = msg_send![
            &center,
            addObserver: &*delegate,
            selector: sel!(didFinishLaunching:),
            name: &*name,
            object: None::<&AnyObject>
        ];
        // Neither the center nor the application keeps the delegate, and it
        // has to answer for the whole life of the app.
        std::mem::forget(delegate);
    }
}
