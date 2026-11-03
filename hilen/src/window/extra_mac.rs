//! What only macOS does for an extra window over everything, see
//! `window/extra`.

use anyhow::{Context, Result, bail};
use log::{debug, warn};
use objc2::{
    ClassType, MainThreadOnly, define_class, msg_send,
    rc::Retained,
    runtime::{AnyClass, AnyObject, Bool},
};
use objc2_app_kit::{
    NSApplicationActivationOptions, NSPanel, NSResponder, NSRunningApplication, NSScreenSaverWindowLevel,
    NSView, NSWindow, NSWindowCollectionBehavior, NSWindowStyleMask, NSWorkspace,
};
use objc2_foundation::NSObject;
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

/// The app that had the focus before a window over everything took it.
pub(crate) struct FrontApp(Retained<NSRunningApplication>);

impl FrontApp {
    /// The app in front, `None` when it is this app.
    pub(crate) fn now() -> Option<Self> {
        let front = NSWorkspace::sharedWorkspace().frontmostApplication()?;
        let own = NSRunningApplication::currentApplication();
        if front.processIdentifier() == own.processIdentifier() {
            return None;
        }
        debug!("The app in front is {:?}", front.localizedName());
        Some(Self(front))
    }

    pub(crate) fn activate(self) {
        if !self.0.activateWithOptions(NSApplicationActivationOptions::empty()) {
            warn!("{:?} did not take the focus back", self.0.localizedName());
        }
    }
}

fn ns_window(window: &winit::window::Window) -> Result<Retained<NSWindow>> {
    let RawWindowHandle::AppKit(handle) = window.window_handle()?.as_raw() else {
        bail!("A macOS window has no AppKit handle");
    };
    // SAFETY: an AppKit handle holds the NSView of the window, which lives as
    // long as the window does.
    let view: &NSView = unsafe { handle.ns_view.cast().as_ref() };
    view.window().context("The view of the window is in no window")
}

define_class!(
    /// The window over everything as a panel that takes the keys and never
    /// activates the app. An app that activates leaves an app in full
    /// screen for the desktop of its own windows, and the window showed
    /// there. `WinitWindow` adds no fields to `NSWindow`, so its class can
    /// be swapped for this one, the way tauri-nspanel does it.
    #[unsafe(super(NSPanel, NSWindow, NSResponder, NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "HilenOverPanel"]
    struct OverPanel;

    impl OverPanel {
        #[unsafe(method(canBecomeKeyWindow))]
        fn can_become_key_window(&self) -> bool {
            true
        }

        #[unsafe(method(canBecomeMainWindow))]
        fn can_become_main_window(&self) -> bool {
            false
        }
    }
);

/// Shows the hidden window on the desktop of the user, an app in full
/// screen included, and gives it the keys without activating the app.
pub(crate) fn show_here(window: &winit::window::Window) -> Result<()> {
    let ns_window = ns_window(window)?;
    ns_window.orderFrontRegardless();
    ns_window.makeKeyWindow();
    Ok(())
}

/// The window stands over the menu bar, the dock and every other window,
/// on every desktop and over an app in full screen.
/// Gives back the class the window had, which `put_class_back` needs.
pub(crate) fn raise_above_all(window: &winit::window::Window) -> Result<&'static AnyClass> {
    let ns_window = ns_window(window)?;
    let object: &AnyObject = &ns_window;
    // SAFETY: `WinitWindow` declares no fields, a panel has the same layout.
    let original = unsafe { AnyObject::set_class(object, OverPanel::class()) };
    ns_window.setStyleMask(ns_window.styleMask() | NSWindowStyleMask::NonactivatingPanel);
    // AppKit reads the style mask of a panel only when it is made, this
    // private call is how an existing panel stops activating its app.
    let _: () = unsafe { msg_send![&*ns_window, _setPreventsActivation: Bool::YES] };
    ns_window.setLevel(NSScreenSaverWindowLevel);
    ns_window.setCollectionBehavior(
        NSWindowCollectionBehavior::CanJoinAllSpaces
            | NSWindowCollectionBehavior::FullScreenAuxiliary
            | NSWindowCollectionBehavior::Stationary
            | NSWindowCollectionBehavior::IgnoresCycle,
    );
    ns_window.setHasShadow(false);
    Ok(original)
}

/// The window gets its class back before winit frees it. The class it had
/// is the hidden subclass that holds the observers of winit, and with the
/// panel class in its place the removal of an observer threw and the app
/// aborted.
pub(crate) fn put_class_back(window: &winit::window::Window, original: &'static AnyClass) {
    match ns_window(window) {
        Ok(ns_window) => {
            let object: &AnyObject = &ns_window;
            // SAFETY: the class the window had before `raise_above_all`.
            unsafe { AnyObject::set_class(object, original) };
        }
        Err(err) => warn!("The panel keeps its class, the app can abort at its end: {err:#}"),
    }
}
