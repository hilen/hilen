#![cfg(target_arch = "wasm32")]

use std::{panic::PanicHookInfo, sync::OnceLock};

static PANIC_BEACON_URL: OnceLock<String> = OnceLock::new();

/// Sends panics to console.error and, when a driver flag is in the url, to
/// the driver's `/te-panic` endpoint. The origin is captured now, on the main
/// thread, because a worker that panics later has no `window` to ask.
pub(crate) fn install_panic_hook() {
    if query_flag("te_inspect") || query_flag("te_run_tests") {
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
        return;
    };

    let Ok(request) = web_sys::XmlHttpRequest::new() else {
        log::error!("Failed to create a panic beacon request");
        return;
    };

    let thread = std::thread::current();
    let body = format!("{}: {info}", thread.name().unwrap_or("unnamed thread"));

    if request.open_with_async("POST", url, false).is_err() || request.send_with_opt_str(Some(&body)).is_err()
    {
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

/// Value of `name` in the page query string.
pub(crate) fn query_param(name: &str) -> Option<String> {
    page_search().trim_start_matches('?').split('&').find_map(|pair| {
        let (key, value) = pair.split_once('=')?;
        (key == name).then(|| value.to_string())
    })
}

fn page_search() -> String {
    web_sys::window()
        .expect("Failed to get browser window")
        .location()
        .search()
        .expect("Failed to get location search")
}
