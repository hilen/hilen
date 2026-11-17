//! `UserNotifications` and the app delegate on iOS. The permission comes
//! from the notification center, the token from the delegate of the app,
//! see `system::ios_delegate`, and a tap from the delegate of the center.

use std::{fs::read, mem::forget};

use anyhow::{Result, anyhow};
use block2::{DynBlock, RcBlock};
use log::{debug, warn};
use objc2::{
    MainThreadMarker, MainThreadOnly, define_class, msg_send,
    rc::Retained,
    runtime::{Bool, NSObject, NSObjectProtocol, ProtocolObject},
};
use objc2_foundation::{NSBundle, NSData, NSError, NSJSONSerialization, NSJSONWritingOptions};
use objc2_ui_kit::UIApplication;
use objc2_user_notifications::{
    UNAuthorizationOptions, UNNotification, UNNotificationDefaultActionIdentifier,
    UNNotificationPresentationOptions, UNNotificationResponse, UNUserNotificationCenter,
    UNUserNotificationCenterDelegate,
};
use parking_lot::Mutex;

use crate::{
    deps::hreads::on_main,
    push::{Done, PushToken, profile_is_sandbox, receive_open, receive_token},
};

define_class!(
    // SAFETY: NSObject has no subclassing requirements and the class has no
    // Drop.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "HilenPushDelegate"]
    struct CenterDelegate;

    unsafe impl NSObjectProtocol for CenterDelegate {}

    unsafe impl UNUserNotificationCenterDelegate for CenterDelegate {
        // iOS hides a push that comes while the app is in front, unless the
        // delegate asks for it.
        #[unsafe(method(userNotificationCenter:willPresentNotification:withCompletionHandler:))]
        fn will_present(
            &self,
            _center: &UNUserNotificationCenter,
            _notification: &UNNotification,
            done: &DynBlock<dyn Fn(UNNotificationPresentationOptions)>,
        ) {
            debug!("a push came while the app is in front, it shows as a banner");
            done.call((UNNotificationPresentationOptions::Banner
                | UNNotificationPresentationOptions::List
                | UNNotificationPresentationOptions::Sound,));
        }

        #[unsafe(method(userNotificationCenter:didReceiveNotificationResponse:withCompletionHandler:))]
        fn did_receive(
            &self,
            _center: &UNUserNotificationCenter,
            response: &UNNotificationResponse,
            done: &DynBlock<dyn Fn()>,
        ) {
            // SAFETY: a constant of the framework.
            let open = unsafe { UNNotificationDefaultActionIdentifier };
            if response.actionIdentifier().isEqualToString(open) {
                match payload(&response.notification()) {
                    Ok(json) => receive_open(json),
                    Err(err) => warn!("the payload of a tapped push was not read: {err:#}"),
                }
            } else {
                debug!("a push was dismissed, {}", response.actionIdentifier());
            }
            done.call(());
        }
    }
);

/// The launch is over. The delegate of the center must be set before the
/// launch ends, or the tap that started the app is lost.
pub(crate) fn launched(mtm: MainThreadMarker) {
    // SAFETY: `init` of NSObject.
    let delegate: Retained<CenterDelegate> = unsafe { msg_send![CenterDelegate::alloc(mtm), init] };
    UNUserNotificationCenter::currentNotificationCenter()
        .setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
    // The center keeps its delegate weak, and it has to answer for the whole
    // life of the app.
    forget(delegate);
}

pub(super) fn request(done: Done) {
    // The block is `Fn`, the answer comes once.
    let pending = Mutex::new(Some(done));
    let block = RcBlock::new(move |granted: Bool, error: *mut NSError| {
        let Some(done) = pending.lock().take() else {
            return;
        };
        // SAFETY: the error is nil or a valid object for the call.
        if let Some(error) = unsafe { error.as_ref() } {
            return done(Err(anyhow!("{}", error.localizedDescription())));
        }
        if granted.as_bool() {
            on_main(register);
        }
        done(Ok(granted.as_bool()));
    });
    let options =
        UNAuthorizationOptions::Alert | UNAuthorizationOptions::Sound | UNAuthorizationOptions::Badge;
    UNUserNotificationCenter::currentNotificationCenter()
        .requestAuthorizationWithOptions_completionHandler(options, &block);
}

fn register() {
    let Some(mtm) = MainThreadMarker::new() else {
        warn!("the push registration needs the main thread");
        return;
    };
    UIApplication::sharedApplication(mtm).registerForRemoteNotifications();
}

pub(crate) fn registered(token: &NSData) {
    receive_token(PushToken {
        hex:     hex::encode(token.to_vec()),
        sandbox: sandbox(),
    });
}

pub(crate) fn register_failed(error: &NSError) {
    warn!("APNs gave the app no token: {}", error.localizedDescription());
}

/// The simulator always has a token of the sandbox. On a device the
/// profile of the build says it. A build of the store, also one for
/// testers, has no profile in the app and uses production.
fn sandbox() -> bool {
    if cfg!(any(target_arch = "x86_64", target_abi = "sim")) {
        return true;
    }
    let path = format!("{}/embedded.mobileprovision", NSBundle::mainBundle().bundlePath());
    match read(&path) {
        Ok(profile) => profile_is_sandbox(&profile),
        Err(err) => {
            debug!("no profile in the app, the token is of production: {err}");
            false
        }
    }
}

/// The user info of a push as JSON, the `aps` dictionary included.
fn payload(notification: &UNNotification) -> Result<String> {
    let info = notification.request().content().userInfo();
    // SAFETY: the user info of a remote push came from JSON, so it is a
    // valid JSON object, and `isValidJSONObject` checks it first. An invalid
    // one would raise an exception of Objective-C.
    unsafe {
        if !NSJSONSerialization::isValidJSONObject(&info) {
            return Err(anyhow!("the payload is no JSON object"));
        }
        let data =
            NSJSONSerialization::dataWithJSONObject_options_error(&info, NSJSONWritingOptions::empty())
                .map_err(|err| anyhow!("{}", err.localizedDescription()))?;
        Ok(String::from_utf8(data.to_vec())?)
    }
}
