//! An iPhone opens the app for a universal link, an https link of a domain
//! that `[links]` of `hilen.toml` names, and for a link of the url scheme.
//! `hilen-mobile` writes the associated domains entitlement and the scheme.
//! iOS gives such a link to the delegate of the app, as
//! `application:continueUserActivity:restorationHandler:` and
//! `application:openURL:options:`. The delegate is in `ios_delegate`.
//!
//! The link of a cold start is in the launch notification. iOS may give the
//! same link to the delegate again after it, that second copy is dropped.

use log::{debug, warn};
use objc2::{msg_send, rc::Retained, runtime::AnyObject};
use objc2_foundation::{NSString, NSURL};

use crate::deps::refs::main_lock::MainLock;

/// The link of a cold start, taken from the launch notification, until the
/// delegate gets its copy.
static LAUNCH_LINK: MainLock<Option<String>> = MainLock::new();

const BROWSING_WEB: &str = "NSUserActivityTypeBrowsingWeb";

/// The launch is over, `notification` is the one of it.
pub(crate) fn launched(notification: &AnyObject) {
    if let Some(url) = launch_link(notification) {
        *LAUNCH_LINK.get_mut() = Some(url.clone());
        super::receive(url, "the launch of iOS");
    }
}

pub(crate) fn open_url(url: &NSURL) {
    match url.absoluteString() {
        Some(url) => take(url.to_string(), "openURL of iOS"),
        None => warn!("iOS opened the app for a link with no text"),
    }
}

/// `false` for a user activity that is no link.
pub(crate) fn continue_activity(activity: &AnyObject) -> bool {
    match activity_link(activity) {
        Some(url) => {
            take(url, "a universal link of iOS");
            true
        }
        None => false,
    }
}

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
