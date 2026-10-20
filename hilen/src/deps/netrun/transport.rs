//! A transport of the app for 1 origin. While one is registered, every
//! `Call` and every `WebSocket` to a URL of that origin goes to it instead
//! of the network, like over a connection the server opened to the app.

use std::{
    collections::BTreeMap,
    future::Future,
    pin::Pin,
    sync::{Arc, PoisonError, RwLock},
};

use reqwest::StatusCode;

use crate::deps::netrun::{rest::Method, ws::WsEvent};

#[cfg(not_wasm)]
pub type TransportFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;
#[cfg(wasm)]
pub type TransportFuture<T> = Pin<Box<dyn Future<Output = T>>>;

/// Where a transported socket reports, the same events a `WebSocket` gives.
#[cfg(not_wasm)]
pub type SocketEvents = Box<dyn FnMut(WsEvent) + Send>;
#[cfg(wasm)]
pub type SocketEvents = Box<dyn FnMut(WsEvent)>;

/// A request as it would go out, with the content type already in the
/// headers.
#[derive(Debug, Clone)]
pub struct TransportRequest {
    pub method:  Method,
    pub url:     String,
    pub headers: BTreeMap<String, String>,
    pub body:    Option<Vec<u8>>,
}

/// The answer to a request. A failing status is no error here, `Call`
/// reads it the same way as an answer from the network.
#[derive(Debug, Clone)]
pub struct TransportAnswer {
    pub status: StatusCode,
    pub body:   String,
}

pub trait Transport: Send + Sync {
    /// An error is a request that did not reach the other side or got no
    /// answer, `RequestError::Transport` for the caller.
    fn request(&self, request: TransportRequest) -> TransportFuture<Result<TransportAnswer, String>>;

    /// A socket to the URL. It reports `Opened` first and `Closed` last,
    /// like a `WebSocket`, and the messages sent before `Opened` wait.
    fn socket(&self, url: String, on_event: SocketEvents) -> Box<dyn TransportSocket>;
}

/// The sending half of a transported socket. Dropping it ends the socket
/// with no `Closed` event, like a dropped `WebSocket`.
pub trait TransportSocket: Send + Sync {
    fn send(&self, text: String);
    fn send_bytes(&self, bytes: Vec<u8>);
    fn close(&self);
}

static TRANSPORTS: RwLock<Vec<(String, Arc<dyn Transport>)>> = RwLock::new(Vec::new());

/// From now on the requests and sockets to `origin`, like
/// `https://banda.example`, go to `transport`. A transport registered for
/// the same origin before is replaced. A socket that is open already stays
/// where it is.
pub fn register_transport(origin: impl AsRef<str>, transport: Arc<dyn Transport>) {
    let origin = origin_of(origin.as_ref());
    let mut transports = TRANSPORTS.write().unwrap_or_else(PoisonError::into_inner);
    transports.retain(|(known, _)| *known != origin);
    log::info!("requests to {origin} go to a transport of the app");
    transports.push((origin, transport));
}

/// The requests to `origin` go to the network again.
pub fn remove_transport(origin: impl AsRef<str>) {
    let origin = origin_of(origin.as_ref());
    let mut transports = TRANSPORTS.write().unwrap_or_else(PoisonError::into_inner);
    let before = transports.len();
    transports.retain(|(known, _)| *known != origin);
    if transports.len() < before {
        log::info!("requests to {origin} go to the network again");
    }
}

/// The transport for a URL, `None` for one that goes to the network.
pub(crate) fn transport_for(url: &str) -> Option<Arc<dyn Transport>> {
    let transports = TRANSPORTS.read().unwrap_or_else(PoisonError::into_inner);
    if transports.is_empty() {
        return None;
    }
    let url = http_scheme(url);
    transports
        .iter()
        .find(|(origin, _)| {
            url.get(..origin.len()).is_some_and(|start| start.eq_ignore_ascii_case(origin))
                && matches!(url.as_bytes().get(origin.len()), None | Some(b'/' | b'?' | b'#'))
        })
        .map(|(_, transport)| transport.clone())
}

/// A socket URL has the scheme of its origin, `wss` for `https`.
fn http_scheme(url: &str) -> String {
    for (socket, http) in [("wss://", "https://"), ("ws://", "http://")] {
        if url.get(..socket.len()).is_some_and(|start| start.eq_ignore_ascii_case(socket)) {
            return format!("{http}{}", &url[socket.len()..]);
        }
    }
    url.to_owned()
}

fn origin_of(origin: &str) -> String {
    http_scheme(origin.trim_end_matches('/')).to_ascii_lowercase()
}

#[cfg(test)]
mod test {
    use super::{http_scheme, origin_of};

    #[test]
    fn a_socket_url_has_the_scheme_of_its_origin() {
        assert_eq!(http_scheme("wss://a.example/api/ws"), "https://a.example/api/ws");
        assert_eq!(http_scheme("WS://a.example"), "http://a.example");
        assert_eq!(http_scheme("https://a.example"), "https://a.example");
        assert_eq!(origin_of("HTTPS://A.example/"), "https://a.example");
    }

    #[cfg(not_wasm)]
    mod not_wasm_test {
        use std::{
            sync::{Arc, Mutex, mpsc::channel},
            time::Duration,
        };

        use anyhow::Result;
        use pretty_assertions::assert_eq;
        use reqwest::StatusCode;
        use serde_json::{Value, json};

        use crate::deps::netrun::{
            rest::{Call, RequestError},
            test_server::{User, start_test_server},
            transport::{
                SocketEvents, Transport, TransportAnswer, TransportFuture, TransportRequest, TransportSocket,
                register_transport, remove_transport, transport_for,
            },
            ws::{WebSocket, WsEvent},
        };

        const TIMEOUT: Duration = Duration::from_secs(10);

        /// Answers each request with what it saw, the path picks a status.
        struct Echo;

        impl Transport for Echo {
            fn request(&self, request: TransportRequest) -> TransportFuture<Result<TransportAnswer, String>> {
                Box::pin(async move {
                    if request.url.ends_with("/broken") {
                        return Err("the line is down".to_owned());
                    }
                    let status = match request.url.rsplit('/').next() {
                        Some("401") => StatusCode::UNAUTHORIZED,
                        Some("503") => StatusCode::SERVICE_UNAVAILABLE,
                        _ => StatusCode::OK,
                    };
                    let seen = json!({
                        "method": request.method.to_string(),
                        "url": request.url,
                        "type": request.headers.get("content-type"),
                        "token": request.headers.get("authorization"),
                        "body": String::from_utf8(request.body.unwrap_or_default()).unwrap_or_default(),
                    });
                    Ok(TransportAnswer {
                        status,
                        body: seen.to_string(),
                    })
                })
            }

            fn socket(&self, url: String, mut on_event: SocketEvents) -> Box<dyn TransportSocket> {
                on_event(WsEvent::Opened);
                on_event(WsEvent::Message(format!("hello {url}")));
                Box::new(EchoSocket {
                    on_event: Mutex::new(on_event),
                })
            }
        }

        struct EchoSocket {
            on_event: Mutex<SocketEvents>,
        }

        impl TransportSocket for EchoSocket {
            fn send(&self, text: String) {
                (self.on_event.lock().unwrap())(WsEvent::Message(text));
            }

            fn send_bytes(&self, bytes: Vec<u8>) {
                (self.on_event.lock().unwrap())(WsEvent::Binary(bytes));
            }

            fn close(&self) {
                (self.on_event.lock().unwrap())(WsEvent::Closed);
            }
        }

        #[tokio::test]
        async fn a_call_to_the_origin_goes_to_the_transport() -> Result<()> {
            register_transport("https://call.transport.invalid/", Arc::new(Echo));

            let seen: Value = Call::post("https://call.transport.invalid/api/send")
                .bearer("abc")
                .body(json!({"text": "hi"}))
                .send()
                .await?;

            assert_eq!(
                seen,
                json!({
                    "method": "POST",
                    "url": "https://call.transport.invalid/api/send",
                    "type": "application/json",
                    "token": "Bearer abc",
                    "body": "{\"text\":\"hi\"}",
                })
            );
            remove_transport("https://call.transport.invalid");
            Ok(())
        }

        #[tokio::test]
        async fn a_status_of_the_transport_reads_like_one_from_the_network() {
            register_transport("https://status.transport.invalid", Arc::new(Echo));

            let error = Call::get("https://status.transport.invalid/401")
                .send::<Value>()
                .await
                .unwrap_err();
            assert!(matches!(error, RequestError::Unauthorized));

            let error = Call::get("https://status.transport.invalid/503")
                .send::<Value>()
                .await
                .unwrap_err();
            assert_eq!(error.status(), Some(StatusCode::SERVICE_UNAVAILABLE));

            let error = Call::get("https://status.transport.invalid/broken")
                .send::<Value>()
                .await
                .unwrap_err();
            assert!(matches!(error, RequestError::Transport(ref why) if why == "the line is down"));
            remove_transport("https://status.transport.invalid");
        }

        /// Another origin, and a host that only starts like the origin,
        /// still go to the network.
        #[tokio::test]
        async fn other_origins_go_to_the_network() -> Result<()> {
            let base_url = start_test_server().await;
            register_transport("http://127.0.0.1", Arc::new(Echo));
            register_transport("https://near.transport.invalid", Arc::new(Echo));

            let users: Vec<User> = Call::get(format!("{base_url}/users")).send().await?;
            assert_eq!(users.len(), 10);
            assert!(transport_for("https://near.transport.invalid.other/x").is_none());
            assert!(transport_for("https://near.transport.invalid:8443/x").is_none());
            assert!(transport_for("https://near.transport.invalid?x=1").is_some());

            remove_transport("http://127.0.0.1");
            remove_transport("https://near.transport.invalid");
            Ok(())
        }

        #[tokio::test]
        async fn a_removed_transport_carries_nothing() {
            register_transport("https://gone.transport.invalid", Arc::new(Echo));
            remove_transport("https://gone.transport.invalid");

            let error = Call::get("https://gone.transport.invalid/x").send::<Value>().await.unwrap_err();
            assert!(matches!(error, RequestError::Send(_)), "{error:?}");
        }

        #[test]
        fn a_socket_to_the_origin_goes_to_the_transport() -> Result<()> {
            register_transport("https://socket.transport.invalid", Arc::new(Echo));
            let (events, received) = channel();

            let socket = WebSocket::connect("wss://socket.transport.invalid/api/ws/app", move |event| {
                events.send(event).expect("Failed to deliver an event");
            });
            socket.send("first");
            socket.send_bytes(vec![0, 255]);
            socket.close();

            assert_eq!(received.recv_timeout(TIMEOUT)?, WsEvent::Opened);
            assert_eq!(
                received.recv_timeout(TIMEOUT)?,
                WsEvent::Message("hello wss://socket.transport.invalid/api/ws/app".to_owned())
            );
            assert_eq!(
                received.recv_timeout(TIMEOUT)?,
                WsEvent::Message("first".to_owned())
            );
            assert_eq!(received.recv_timeout(TIMEOUT)?, WsEvent::Binary(vec![0, 255]));
            assert_eq!(received.recv_timeout(TIMEOUT)?, WsEvent::Closed);
            remove_transport("https://socket.transport.invalid");
            Ok(())
        }
    }
}
