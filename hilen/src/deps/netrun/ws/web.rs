use std::{cell::RefCell, rc::Rc};

use log::{debug, error, warn};
use web_sys::{
    MessageEvent,
    wasm_bindgen::{JsCast, closure::Closure},
};

use crate::deps::netrun::ws::WsEvent;

type Callback = Rc<RefCell<dyn FnMut(WsEvent)>>;

/// The browser side of the client. The socket and its JS callbacks live
/// on the main thread. Dropping the handle detaches the callbacks and
/// closes the socket without a Closed event.
pub struct WebSocket {
    socket:   Option<web_sys::WebSocket>,
    handlers: Option<Handlers>,
    pending:  Rc<RefCell<Vec<String>>>,
}

struct Handlers {
    open:    Closure<dyn FnMut()>,
    message: Closure<dyn FnMut(MessageEvent)>,
    error:   Closure<dyn FnMut()>,
    close:   Closure<dyn FnMut()>,
}

impl WebSocket {
    pub fn connect(url: impl ToString, on_event: impl FnMut(WsEvent) + 'static) -> Self {
        let url = url.to_string();
        let callback: Callback = Rc::new(RefCell::new(on_event));
        let pending: Rc<RefCell<Vec<String>>> = Rc::default();

        let socket = match web_sys::WebSocket::new(&url) {
            Ok(socket) => socket,
            Err(err) => {
                emit(
                    &callback,
                    WsEvent::Error(format!("Failed to open WebSocket at {url}: {err:?}")),
                );
                emit(&callback, WsEvent::Closed);
                return Self {
                    socket: None,
                    handlers: None,
                    pending,
                };
            }
        };

        let open = {
            let callback = callback.clone();
            let pending = pending.clone();
            let socket = socket.clone();

            Closure::<dyn FnMut()>::new(move || {
                emit(&callback, WsEvent::Opened);

                for message in pending.borrow_mut().drain(..) {
                    if let Err(err) = socket.send_with_str(&message) {
                        error!("Failed to flush a queued WebSocket message: {err:?}");
                    }
                }
            })
        };

        let message = {
            let callback = callback.clone();

            Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
                let Some(text) = event.data().as_string() else {
                    warn!("WebSocket ignored a non text frame");
                    return;
                };

                emit(&callback, WsEvent::Message(text));
            })
        };

        let error = {
            let callback = callback.clone();

            // The browser reports no detail for socket errors.
            Closure::<dyn FnMut()>::new(move || {
                emit(&callback, WsEvent::Error("WebSocket failed".to_string()));
            })
        };

        let close = {
            let callback = callback.clone();

            Closure::<dyn FnMut()>::new(move || {
                emit(&callback, WsEvent::Closed);
            })
        };

        let handlers = Handlers {
            open,
            message,
            error,
            close,
        };

        socket.set_onopen(Some(handlers.open.as_ref().unchecked_ref()));
        socket.set_onmessage(Some(handlers.message.as_ref().unchecked_ref()));
        socket.set_onerror(Some(handlers.error.as_ref().unchecked_ref()));
        socket.set_onclose(Some(handlers.close.as_ref().unchecked_ref()));

        Self {
            socket: Some(socket),
            handlers: Some(handlers),
            pending,
        }
    }

    pub fn send(&self, message: impl ToString) {
        let Some(socket) = &self.socket else {
            warn!("WebSocket message dropped, the connection never opened");
            return;
        };

        let message = message.to_string();

        match socket.ready_state() {
            web_sys::WebSocket::CONNECTING => self.pending.borrow_mut().push(message),
            web_sys::WebSocket::OPEN => {
                if let Err(err) = socket.send_with_str(&message) {
                    error!("Failed to send over WebSocket: {err:?}");
                }
            }
            _ => warn!("WebSocket message dropped, the connection is closed"),
        }
    }

    pub fn close(&self) {
        let Some(socket) = &self.socket else {
            return;
        };

        if let Err(err) = socket.close() {
            debug!("WebSocket close failed: {err:?}");
        }
    }
}

impl Drop for WebSocket {
    fn drop(&mut self) {
        let Some(socket) = &self.socket else {
            return;
        };

        socket.set_onopen(None);
        socket.set_onmessage(None);
        socket.set_onerror(None);
        socket.set_onclose(None);

        // The JS handlers must not die while still registered, so they are
        // dropped only after the socket forgot them.
        drop(self.handlers.take());

        let state = socket.ready_state();

        if (state == web_sys::WebSocket::CONNECTING || state == web_sys::WebSocket::OPEN)
            && let Err(err) = socket.close()
        {
            debug!("WebSocket close on drop failed: {err:?}");
        }
    }
}

fn emit(callback: &Callback, event: WsEvent) {
    let mut callback = callback.borrow_mut();
    (*callback)(event);
}
