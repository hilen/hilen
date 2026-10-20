//! The inspect channel for a browser page. A page cannot listen on TCP and
//! has no mDNS, so the direction inverts: when the `hilen_inspect` query flag
//! is set, the app dials out to the server that served the page, same origin,
//! at `/hilen-inspect`. The protocol stays the same request in, response out
//! JSON, one WebSocket text message per frame instead of a length prefix.
//!
//! An app installed on a TV opens from a file, it has no server to dial and
//! no query. Its dev build bakes the address in, `HILEN_INSPECT_URL`, and
//! dials it at every start and again whenever the socket closes.

use std::{
    hint::black_box,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{Sender, channel},
    },
};

use parking_lot::Mutex;
use web_sys::{
    MessageEvent, WebSocket,
    wasm_bindgen::{JsCast, closure::Closure},
};

use crate::{
    deps::{hreads::on_main, refs::main_lock::MainLock},
    inspect::{
        InspectService, MARKER,
        protocol::{AppCommand, InspectorCommand},
    },
    ui::UIManager,
};

/// The address the dev build of an installed app dials, set when it is
/// compiled. See "An installed TV app" in `docs/inspect.md`.
const BAKED_URL: Option<&str> = option_env!("HILEN_INSPECT_URL");
/// A baked address is dialed again this long after its socket closed, the
/// tunnel to it can come up after the app.
const REDIAL_MS: i32 = 3000;

/// Lives on the main thread, where the socket's JS callbacks fire.
static SOCKET: MainLock<Option<WebSocket>> = MainLock::new();

/// Requests cross from the socket callbacks to the inspect worker here.
static REQUESTS: Mutex<Option<Sender<InspectorCommand>>> = Mutex::new(None);

/// Whether the socket is open, readable from any thread. The log
/// forwarder asks before every line, and it must not touch the socket
/// itself, which lives on the main thread.
static OPEN: AtomicBool = AtomicBool::new(false);

pub(crate) fn is_open() -> bool {
    OPEN.load(Ordering::Relaxed)
}

pub(crate) fn start_if_requested() {
    black_box(MARKER);

    let Some(url) = BAKED_URL.map(str::to_string).or_else(page_url) else {
        return;
    };

    let (sender, receiver) = channel::<InspectorCommand>();
    *REQUESTS.lock() = Some(sender);

    // One long lived worker processes requests in order. Commands block on
    // `from_main` and `RunTests` runs the whole suite, neither may happen on
    // the main thread.
    crate::deps::hreads::spawn_thread(move || {
        while let Ok(request) = receiver.recv() {
            let response = InspectService::process_command(request);
            let data = serde_json::to_string(&response);

            // The response can hold Own pointers, which must drop on the main
            // thread.
            on_main(move || drop(response));

            match data {
                Ok(frame) => send_frame(frame, false),
                Err(err) => log::error!("Failed to serialize inspect response: {err}"),
            }
        }
    });

    dial(url);
}

/// The server that served the page, when the url asks for the inspector.
fn page_url() -> Option<String> {
    if !crate::web::query_flag("hilen_inspect") {
        return None;
    }
    let location = web_sys::window().expect("Failed to get browser window").location();
    let protocol = location.protocol().expect("Failed to get location protocol");
    let host = location.host().expect("Failed to get location host");
    let scheme = if protocol == "https:" { "wss" } else { "ws" };
    Some(format!("{scheme}://{host}/hilen-inspect"))
}

/// Opens the socket. A baked address is dialed again after a failure.
fn dial(url: String) {
    let socket = match WebSocket::new(&url) {
        Ok(socket) => socket,
        Err(err) => {
            log::error!("Failed to open inspect socket at {url}: {err:?}");
            redial(url);
            return;
        }
    };

    let onmessage = Closure::<dyn FnMut(MessageEvent)>::new(|event: MessageEvent| {
        let Some(text) = event.data().as_string() else {
            log::error!("Inspect socket received a non text frame");
            return;
        };

        match serde_json::from_str::<InspectorCommand>(&text) {
            Ok(command) => {
                let failed = REQUESTS.lock().as_ref().is_none_or(|sender| sender.send(command).is_err());
                if failed {
                    log::error!("Inspect worker is gone, dropping a request");
                }
            }
            Err(err) => log::error!("Failed to parse inspect request: {err}"),
        }
    });
    socket.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
    // The handler lives as long as the page.
    onmessage.forget();

    let onopen = Closure::<dyn FnMut()>::new(|| {
        OPEN.store(true, Ordering::Relaxed);
        log::info!("Inspect socket connected");
        hello();
    });
    socket.set_onopen(Some(onopen.as_ref().unchecked_ref()));
    onopen.forget();

    // A failed dial closes too, so a baked address that is not up yet is
    // tried again every few seconds. Only the close of an open socket says
    // so, the tries would fill the log.
    let onclose = Closure::<dyn FnMut()>::new(move || {
        if OPEN.swap(false, Ordering::Relaxed) {
            log::warn!("Inspect socket closed");
        }
        redial(url.clone());
    });
    socket.set_onclose(Some(onclose.as_ref().unchecked_ref()));
    onclose.forget();

    SOCKET.set(Some(socket));
}

/// Dials a baked address again after a pause. A page that dials its own
/// server stays closed, its driver opens the page again.
fn redial(url: String) {
    if BAKED_URL.is_none() {
        return;
    }
    let Some(window) = web_sys::window() else {
        return;
    };
    let again = Closure::once_into_js(move || dial(url));
    if let Err(err) =
        window.set_timeout_with_callback_and_timeout_and_arguments_0(again.unchecked_ref(), REDIAL_MS)
    {
        log::error!("Failed to plan the next inspect dial: {err:?}");
    }
}

/// Tells the other side which app this is, by its id and the title of the
/// page, so a bridge can list it like a native app.
fn hello() {
    let name = web_sys::window()
        .and_then(|window| window.document())
        .map(|document| document.title())
        .filter(|title| !title.is_empty())
        .unwrap_or_else(|| "page".to_string());
    push(AppCommand::Hello {
        app_id: UIManager::app_instance_id().to_string(),
        name,
    });
}

/// Sends a frame the driver did not request, the test autorun delivers its
/// report this way. Quietly does nothing when the channel is not up.
pub(crate) fn push(command: AppCommand) {
    on_main(move || match serde_json::to_string(&command) {
        Ok(frame) => send_frame(frame, false),
        Err(err) => log::error!("Failed to serialize inspect push: {err}"),
    });
}

/// `push` for a log line. Says nothing when it fails, a log line about a
/// dropped log line would forward itself without end.
pub(crate) fn push_log(level: &str, message: String) {
    let command = AppCommand::Log {
        level: level.to_string(),
        message,
    };
    on_main(move || {
        if let Ok(frame) = serde_json::to_string(&command) {
            send_frame(frame, true);
        }
    });
}

/// Main thread only, the socket lives there.
fn send_frame(frame: String, quiet: bool) {
    on_main(move || {
        let Some(socket) = SOCKET.get_mut() else {
            if !quiet {
                log::warn!("Inspect frame dropped, no socket");
            }
            return;
        };

        if socket.ready_state() != WebSocket::OPEN {
            if !quiet {
                log::warn!("Inspect frame dropped, socket is not open");
            }
            return;
        }

        if let Err(err) = socket.send_with_str(&frame)
            && !quiet
        {
            log::error!("Failed to send inspect frame: {err:?}");
        }
    });
}
