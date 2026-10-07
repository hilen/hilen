//! The start and the stop of a hot build, see `docs/hot-reload.md`. A loader
//! app owns the process. It loads this library, starts it, and stops it when
//! a newer build is there.

use std::{
    cell::RefCell,
    ffi::{CStr, c_int},
    ptr::from_ref,
    sync::atomic::{AtomicBool, Ordering},
};

use log::info;
use objc2::{ffi::objc_disposeClassPair, runtime::AnyClass};
use parking_lot::Mutex;
use sentry::ClientInitGuard;
use tokio::runtime::{EnterGuard, Handle, Runtime};
use winit::platform::ios::release_classes;

#[cfg(feature = "audio")]
use crate::audio::manager::stop as stop_audio;
#[cfg(feature = "level")]
use crate::level::LevelManager;
#[cfg(feature = "scene")]
use crate::scene::SceneManager;
use crate::{
    App, AppRunner,
    deps::hreads::stop_dispatch,
    filesystem::stop_picker,
    inspect::InspectService,
    system::MediaSession,
    ui::{UIManager, mobile::ios::hilen_ios_text_stop},
    window::AppHandler,
};

static RUNTIME: Mutex<Option<Runtime>> = Mutex::new(None);
static SENTRY: Mutex<Option<ClientInitGuard>> = Mutex::new(None);
static CLASSES_DELETED: AtomicBool = AtomicBool::new(false);

thread_local! {
    static CONTEXT: RefCell<Option<EnterGuard<'static>>> = const { RefCell::new(None) };
}

/// A normal start never leaves the `block_on` of its runtime. A hot start
/// returns, so the runtime, its context on the main thread and the Sentry
/// guard live here. `tokio::spawn` on the main thread needs that context.
pub(crate) fn enter_runtime(app: &dyn App) {
    let runtime = Runtime::new().expect("Failed to start the tokio runtime");

    *SENTRY.lock() = runtime.block_on(AppRunner::setup_sentry(app));

    // The guard borrows the handle, and the guard lives in a thread local.
    let handle: &'static Handle = Box::leak(Box::new(runtime.handle().clone()));
    CONTEXT.with_borrow_mut(|context| *context = Some(handle.enter()));

    *RUNTIME.lock() = Some(runtime);
}

/// The loader calls this before it starts a newer build. The image stays
/// loaded and must run no code again, so this gives back what the OS or
/// another thread would call it through: the views with their players and
/// GPU objects, the system text field, the network, the window, the event
/// loop and the runtime.
#[unsafe(no_mangle)]
pub extern "C" fn hilen_stop() {
    info!("Hot build stops");

    #[cfg(feature = "level")]
    LevelManager::stop_level();
    #[cfg(feature = "scene")]
    SceneManager::stop_scene();

    let mut root = UIManager::root_view();
    root.clear_root();
    UIManager::free_deleted_views();

    // After the views, a video player holds a track of the audio manager.
    #[cfg(feature = "audio")]
    stop_audio();

    MediaSession::stop();
    // SAFETY: the main thread, and the function takes no argument.
    unsafe { hilen_ios_text_stop() };
    stop_picker();
    InspectService::stop();
    stop_dispatch();

    AppHandler::stop();

    SENTRY.lock().take();
    CONTEXT.with_borrow_mut(Option::take);
    if let Some(runtime) = RUNTIME.lock().take() {
        runtime.shutdown_background();
    }
}

/// The classes objc2 made at run time for this build, besides the 3 of winit.
const CLASSES: [&CStr; 2] = [c"RawWindowMetalLayer", c"HilenImagePickerDelegate"];

/// 1 when the Objective-C classes of this build are deleted, so the next
/// build can register the same names. `UIKit` holds a hidden window a moment
/// longer, the loader asks until this is 1.
#[unsafe(no_mangle)]
pub extern "C" fn hilen_stopped() -> c_int {
    if CLASSES_DELETED.load(Ordering::Relaxed) {
        return 1;
    }
    if !release_classes() {
        return 0;
    }

    // The layer of the surface is a sublayer of the winit view, so it is
    // gone when that view is. The picker delegate went in `hilen_stop`.
    for class in CLASSES.iter().filter_map(|name| AnyClass::get(name)) {
        // SAFETY: the class was made at run time and has no object left.
        unsafe { objc_disposeClassPair(from_ref(class).cast_mut().cast()) };
    }

    CLASSES_DELETED.store(true, Ordering::Relaxed);
    1
}
