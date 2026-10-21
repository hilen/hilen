//! The delegate of the app on iOS. winit starts `UIApplicationMain` with no
//! delegate, so the engine sets its own when the launch is over, in the
//! notification of it. The links of the app come to it, see `links`, and
//! with the `push` feature the device token of APNs, see `push`.

use log::{debug, warn};
use objc2::{
    MainThreadMarker, MainThreadOnly, class, define_class, msg_send,
    rc::Retained,
    runtime::{AnyObject, Bool, NSObject},
    sel,
};
#[cfg(feature = "push")]
use objc2_foundation::{NSData, NSError};
use objc2_foundation::{NSString, NSURL};

use crate::system::links::ios as links;

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
                warn!("the app has a delegate already, it gets no link and no push");
                return;
            }
            // SAFETY: the delegate property does not keep the object, and
            // this one is never released, see `install`.
            unsafe {
                let _: () = msg_send![&app, setDelegate: self];
            }

            links::launched(notification);
            #[cfg(feature = "push")]
            crate::push::ios::launched(self.mtm());
        }

        #[unsafe(method(application:openURL:options:))]
        fn open_url(&self, _app: &AnyObject, url: &NSURL, _options: &AnyObject) -> Bool {
            links::open_url(url);
            Bool::YES
        }

        #[unsafe(method(application:continueUserActivity:restorationHandler:))]
        fn continue_activity(&self, _app: &AnyObject, activity: &AnyObject, _restore: *mut AnyObject) -> Bool {
            Bool::new(links::continue_activity(activity))
        }

        #[cfg(feature = "push")]
        #[unsafe(method(application:didRegisterForRemoteNotificationsWithDeviceToken:))]
        fn registered(&self, _app: &AnyObject, token: &NSData) {
            crate::push::ios::registered(token);
        }

        #[cfg(feature = "push")]
        #[unsafe(method(application:didFailToRegisterForRemoteNotificationsWithError:))]
        fn register_failed(&self, _app: &AnyObject, error: &NSError) {
            crate::push::ios::register_failed(error);
        }
    }
);

/// Runs before the event loop starts `UIApplicationMain`, on the main
/// thread. A hot build runs in a loader app that launched long ago and has
/// its own delegate, it gets no link and no push.
pub(crate) fn install() {
    let Some(mtm) = MainThreadMarker::new() else {
        warn!("the app delegate needs the main thread, the app gets no link and no push");
        return;
    };
    // SAFETY: nil before `UIApplicationMain`.
    let running: Option<Retained<AnyObject>> = unsafe { msg_send![class!(UIApplication), sharedApplication] };
    if running.is_some() {
        debug!("the app runs inside another one, it gets no link and no push");
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
