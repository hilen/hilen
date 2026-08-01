//! The inspect channel for a browser page. A page cannot listen on TCP and
//! has no mDNS, so the direction inverts: when the `te_inspect` query flag is
//! set, the app dials out to the server that served the page, same origin,
//! at `/te-inspect`. The protocol stays the same request in, response out
//! JSON, one WebSocket text message per frame instead of a length prefix.

use std::sync::mpsc::{Sender, channel};

use hreads::on_main;
use parking_lot::Mutex;
use refs::main_lock::MainLock;
use web_sys::{
    MessageEvent, WebSocket,
    wasm_bindgen::{JsCast, closure::Closure},
};

use crate::inspect::{
    InspectService,
    protocol::{AppCommand, InspectorCommand},
};

/// Lives on the main thread, where the socket's JS callbacks fire.
static SOCKET: MainLock<Option<WebSocket>> = MainLock::new();

/// Requests cross from the socket callbacks to the inspect worker here.
static REQUESTS: Mutex<Option<Sender<InspectorCommand>>> = Mutex::new(None);

pub(crate) fn start_if_requested() {
    if !crate::web::query_flag("te_inspect") {
        return;
    }

    let location = web_sys::window().expect("Failed to get browser window").location();
    let protocol = location.protocol().expect("Failed to get location protocol");
    let host = location.host().expect("Failed to get location host");
    let scheme = if protocol == "https:" { "wss" } else { "ws" };
    let url = format!("{scheme}://{host}/te-inspect");

    let socket = match WebSocket::new(&url) {
        Ok(socket) => socket,
        Err(err) => {
            log::error!("Failed to open inspect socket at {url}: {err:?}");
            return;
        }
    };

    let (sender, receiver) = channel::<InspectorCommand>();
    *REQUESTS.lock() = Some(sender);

    // One long lived worker processes requests in order. Commands block on
    // `from_main` and `RunTests` runs the whole suite, neither may happen on
    // the main thread.
    hreads::spawn_thread(move || {
        while let Ok(request) = receiver.recv() {
            let response = InspectService::process_command(request);
            let data = serde_json::to_string(&response);

            // The response can hold Own pointers, which must drop on the main thread.
            on_main(move || drop(response));

            match data {
                Ok(frame) => send_frame(frame),
                Err(err) => log::error!("Failed to serialize inspect response: {err}"),
            }
        }
    });

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
        log::info!("Inspect socket connected");
    });
    socket.set_onopen(Some(onopen.as_ref().unchecked_ref()));
    onopen.forget();

    let onclose = Closure::<dyn FnMut()>::new(|| {
        log::warn!("Inspect socket closed");
    });
    socket.set_onclose(Some(onclose.as_ref().unchecked_ref()));
    onclose.forget();

    SOCKET.set(Some(socket));
}

/// Sends a frame the driver did not request, the test autorun delivers its
/// report this way. Quietly does nothing when the channel is not up.
pub(crate) fn push(command: AppCommand) {
    on_main(move || match serde_json::to_string(&command) {
        Ok(frame) => send_frame(frame),
        Err(err) => log::error!("Failed to serialize inspect push: {err}"),
    });
}

/// Main thread only, the socket lives there.
fn send_frame(frame: String) {
    on_main(move || {
        let Some(socket) = SOCKET.get_mut() else {
            log::warn!("Inspect frame dropped, no socket");
            return;
        };

        if socket.ready_state() != WebSocket::OPEN {
            log::warn!("Inspect frame dropped, socket is not open");
            return;
        }

        if let Err(err) = socket.send_with_str(&frame) {
            log::error!("Failed to send inspect frame: {err:?}");
        }
    });
}
