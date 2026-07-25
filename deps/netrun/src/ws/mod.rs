//! A text WebSocket client with one API on every platform. The browser
//! build dials through the JS socket, every other build through tokio.
//! Events arrive on the socket's own context, the browser main thread on
//! wasm and a tokio task on native, so a UI app hops to the main thread
//! with `on_main` before touching views.

#[cfg(not_wasm)]
mod native;
#[cfg(wasm)]
mod web;

#[cfg(not_wasm)]
pub use native::WebSocket;
#[cfg(wasm)]
pub use web::WebSocket;

/// What a connection reports through its event callback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WsEvent {
    /// The connection is up. Messages sent before this were queued and
    /// flush right after.
    Opened,
    /// One incoming text message.
    Message(String),
    /// A transport failure. When the connection dies `Closed` follows.
    Error(String),
    /// The connection ended. Dropping the handle closes silently instead.
    Closed,
}

#[cfg(test)]
mod test {

    #[cfg(not_wasm)]
    mod not_wasm_test {
        use std::{sync::mpsc::channel, time::Duration};

        use anyhow::Result;
        use futures_util::{SinkExt, StreamExt};
        use pretty_assertions::assert_eq;
        use tokio::net::TcpListener;
        use tokio_tungstenite::accept_async;

        use crate::ws::{WebSocket, WsEvent};

        const TIMEOUT: Duration = Duration::from_secs(10);

        #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
        async fn ws_round_trip() -> Result<()> {
            let listener = TcpListener::bind("127.0.0.1:0").await?;
            let address = listener.local_addr()?;

            tokio::spawn(async move {
                let (stream, _) = listener.accept().await.expect("Failed to accept a connection");
                let mut socket = accept_async(stream).await.expect("Failed the WebSocket handshake");

                while let Some(Ok(message)) = socket.next().await {
                    if message.is_text() {
                        socket.send(message).await.expect("Failed to echo a message");
                    }
                }
            });

            let (events, received) = channel();

            let socket = WebSocket::connect(format!("ws://{address}"), move |event| {
                events.send(event).expect("Failed to deliver an event");
            });

            // Sent before the connection opens, must be queued and flushed.
            socket.send("first");

            assert_eq!(received.recv_timeout(TIMEOUT)?, WsEvent::Opened);
            assert_eq!(
                received.recv_timeout(TIMEOUT)?,
                WsEvent::Message("first".to_string())
            );

            socket.send("second");
            assert_eq!(
                received.recv_timeout(TIMEOUT)?,
                WsEvent::Message("second".to_string())
            );

            socket.close();
            assert_eq!(received.recv_timeout(TIMEOUT)?, WsEvent::Closed);

            Ok(())
        }

        /// Same public echo service the wasm test uses. This exercises the
        /// explicit rustls connector, which the local ws:// test never touches.
        #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
        async fn ws_tls_round_trip() -> Result<()> {
            let (events, received) = channel();

            let socket = WebSocket::connect("wss://ws.postman-echo.com/raw", move |event| {
                events.send(event).expect("Failed to deliver an event");
            });

            socket.send("over-tls");

            assert_eq!(received.recv_timeout(TIMEOUT)?, WsEvent::Opened);
            assert_eq!(
                received.recv_timeout(TIMEOUT)?,
                WsEvent::Message("over-tls".to_string())
            );

            socket.close();
            assert_eq!(received.recv_timeout(TIMEOUT)?, WsEvent::Closed);

            Ok(())
        }

        #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
        async fn ws_connect_failure() -> Result<()> {
            let listener = TcpListener::bind("127.0.0.1:0").await?;
            let address = listener.local_addr()?;
            drop(listener);

            let (events, received) = channel();

            let socket = WebSocket::connect(format!("ws://{address}"), move |event| {
                events.send(event).expect("Failed to deliver an event");
            });

            let error = received.recv_timeout(TIMEOUT)?;
            assert!(
                matches!(error, WsEvent::Error(_)),
                "Expected an error event, got {error:?}"
            );
            assert_eq!(received.recv_timeout(TIMEOUT)?, WsEvent::Closed);

            // Closing a dead connection is a quiet no op.
            socket.close();

            Ok(())
        }
    }

    #[cfg(wasm)]
    mod wasm_test {
        use std::{cell::RefCell, rc::Rc};

        use wasm_bindgen_test::wasm_bindgen_test;

        use crate::ws::{WebSocket, WsEvent};

        wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

        /// A public echo service, the same style as the REST tests which
        /// hit a public JSON API.
        const ECHO_URL: &str = "wss://ws.postman-echo.com/raw";

        async fn wait_for_events(events: &Rc<RefCell<Vec<WsEvent>>>, count: usize) {
            for _ in 0..100 {
                if events.borrow().len() >= count {
                    return;
                }
                hreads::sleep(0.1).await;
            }
            panic!(
                "Timed out waiting for {count} WebSocket events, got {:?}",
                events.borrow()
            );
        }

        #[wasm_bindgen_test]
        async fn ws_round_trip() {
            let events: Rc<RefCell<Vec<WsEvent>>> = Rc::default();
            let sink = events.clone();

            let socket = WebSocket::connect(ECHO_URL, move |event| sink.borrow_mut().push(event));

            // Sent before the connection opens, must be queued and flushed.
            socket.send("first");

            wait_for_events(&events, 2).await;
            assert_eq!(
                *events.borrow(),
                vec![WsEvent::Opened, WsEvent::Message("first".to_string())]
            );

            socket.send("second");
            wait_for_events(&events, 3).await;
            assert_eq!(events.borrow()[2], WsEvent::Message("second".to_string()));

            socket.close();
            wait_for_events(&events, 4).await;
            assert_eq!(events.borrow()[3], WsEvent::Closed);
        }
    }
}
