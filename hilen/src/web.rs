#![cfg(target_arch = "wasm32")]

use std::{panic::PanicHookInfo, sync::OnceLock};

use crate::deps::refs::main_lock::MainLock;

static PANIC_BEACON_URL: OnceLock<String> = OnceLock::new();

static RELOAD_SHORTCUT_LISTENER: MainLock<
    Option<web_sys::wasm_bindgen::closure::Closure<dyn FnMut(web_sys::KeyboardEvent)>>,
> = MainLock::new();

/// Winit's canvas keydown handler calls `preventDefault` on every key, which
/// also cancels the browser reload shortcuts while the canvas has focus. This
/// capture phase listener on `window` runs before the canvas handler and stops
/// reload keys there, so nothing cancels them and the browser reloads.
pub(crate) fn install_reload_shortcut_listener() {
    use web_sys::wasm_bindgen::{JsCast, closure::Closure};

    let listener = Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(|event: web_sys::KeyboardEvent| {
        let reload_combo = (event.meta_key() || event.ctrl_key()) && event.code() == "KeyR";
        if reload_combo || event.code() == "F5" {
            event.stop_immediate_propagation();
        }
    });

    let options = web_sys::AddEventListenerOptions::new();
    options.set_capture(true);

    web_sys::window()
        .expect("Failed to get browser window")
        .add_event_listener_with_callback_and_add_event_listener_options(
            "keydown",
            listener.as_ref().unchecked_ref(),
            &options,
        )
        .expect("Failed to install the reload shortcut listener");

    RELOAD_SHORTCUT_LISTENER.set(Some(listener));
}

/// Sends panics to console.error and, when a driver flag is in the url, to
/// the driver's `/te-panic` endpoint. The origin is captured now, on the main
/// thread, because a worker that panics later has no `window` to ask.
pub(crate) fn install_panic_hook() {
    if query_flag("hilen_inspect") || query_flag("hilen_run_tests") {
        let origin = web_sys::window()
            .expect("Failed to get browser window")
            .location()
            .origin()
            .expect("Failed to get location origin");
        PANIC_BEACON_URL
            .set(format!("{origin}/te-panic"))
            .expect("Panic hook installed twice");
    }

    std::panic::set_hook(Box::new(|info| {
        console_error_panic_hook::hook(info);
        report_panic(info);
    }));
}

/// A wasm panic kills the instance right after the hook returns, so the
/// beacon must deliver before the hook does. A sync XHR blocks until sent,
/// and it exists on workers too, where the suite actually runs.
fn report_panic(info: &PanicHookInfo) {
    let Some(url) = PANIC_BEACON_URL.get() else {
        log::error!("Panic beacon has no url, the panic reaches no driver");
        return;
    };

    let Ok(request) = web_sys::XmlHttpRequest::new() else {
        log::error!("Failed to create a panic beacon request");
        return;
    };

    log::error!("Panic beacon sending");

    // Nothing here may touch thread local state. A second panic inside the
    // hook aborts the instance at once, with no console output and no beacon,
    // which is indistinguishable from the driver's own timeout.
    let body = format!("{info}");

    if request.open_with_async("POST", url, false).is_err() {
        log::error!("Failed to send the panic beacon");
        return;
    }

    // The driver relaunches the suite past a panicked test, so it needs the
    // test's name. Only a try lock is safe, the panicking thread may hold it.
    if let Some(name) = crate::ui_test::current_test_name_nonblocking() {
        if request.set_request_header("X-TE-Test", &name).is_err() {
            log::error!("Failed to set the panic beacon test header");
        }
    }

    if request.send_with_opt_str(Some(&body)).is_err() {
        log::error!("Failed to send the panic beacon");
    }
}

/// True when `name` is a key in the page query string. The browser test
/// driver uses query flags the way native lanes use env vars.
pub(crate) fn query_flag(name: &str) -> bool {
    page_search()
        .trim_start_matches('?')
        .split('&')
        .any(|pair| pair == name || pair.split_once('=').is_some_and(|(key, _)| key == name))
}

/// Value of `name` in the page query string. Only the browser test
/// autorun reads parameters, the app facing flags are boolean.
#[cfg(feature = "ui-tests")]
pub(crate) fn query_param(name: &str) -> Option<String> {
    page_search().trim_start_matches('?').split('&').find_map(|pair| {
        let (key, value) = pair.split_once('=')?;
        (key == name).then(|| decode_query_value(value))
    })
}

/// `location.search` keeps the query percent encoded, so a spaced test
/// name arrives as `Reload%20shortcuts%20test` and a raw compare against
/// registered names matches nothing.
#[cfg(feature = "ui-tests")]
fn decode_query_value(value: &str) -> String {
    match web_sys::js_sys::decode_uri_component(value) {
        Ok(decoded) => decoded.into(),
        Err(err) => {
            log::error!("Failed to decode query value {value}: {err:?}");
            value.to_string()
        }
    }
}

fn page_search() -> String {
    web_sys::window()
        .expect("Failed to get browser window")
        .location()
        .search()
        .expect("Failed to get location search")
}
