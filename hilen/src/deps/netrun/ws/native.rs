use std::sync::Arc;

use futures_util::{SinkExt, StreamExt, stream::SplitSink};
use log::{debug, error, warn};
use tokio::{
    net::TcpStream,
    sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel},
};
use tokio_tungstenite::{
    Connector, MaybeTlsStream, WebSocketStream, connect_async_tls_with_config, tungstenite::Message,
};

use crate::deps::netrun::{
    transport::{TransportSocket, transport_for},
    ws::WsEvent,
};

type WriteHalf = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;

/// The tokio side of the client. The connection lives on a spawned task,
/// the handle only feeds it commands, so `send` never blocks. A URL with a
/// transport of the app goes to that transport instead.
pub struct WebSocket {
    link: Link,
}

enum Link {
    Network(UnboundedSender<Command>),
    Transport(Box<dyn TransportSocket>),
}

enum Command {
    Send(String),
    SendBytes(Vec<u8>),
    Close,
}

impl WebSocket {
    pub fn connect(url: impl ToString, on_event: impl FnMut(WsEvent) + Send + 'static) -> Self {
        let url = url.to_string();
        if let Some(transport) = transport_for(&url) {
            return Self {
                link: Link::Transport(transport.socket(url, Box::new(on_event))),
            };
        }

        let (commands, receiver) = unbounded_channel();

        crate::deps::hreads::spawn(run(url, receiver, on_event));

        Self {
            link: Link::Network(commands),
        }
    }

    pub fn send(&self, message: impl ToString) {
        match &self.link {
            Link::Network(commands) => {
                if commands.send(Command::Send(message.to_string())).is_err() {
                    warn!("WebSocket message dropped, the connection is gone");
                }
            }
            Link::Transport(socket) => socket.send(message.to_string()),
        }
    }

    /// Sends a binary message. It keeps its place among the text
    /// messages sent before and after it.
    pub fn send_bytes(&self, bytes: impl Into<Vec<u8>>) {
        match &self.link {
            Link::Network(commands) => {
                if commands.send(Command::SendBytes(bytes.into())).is_err() {
                    warn!("WebSocket bytes dropped, the connection is gone");
                }
            }
            Link::Transport(socket) => socket.send_bytes(bytes.into()),
        }
    }

    pub fn close(&self) {
        match &self.link {
            Link::Network(commands) => {
                if commands.send(Command::Close).is_err() {
                    debug!("WebSocket close skipped, the connection is gone");
                }
            }
            Link::Transport(socket) => socket.close(),
        }
    }
}

async fn run(url: String, mut commands: UnboundedReceiver<Command>, mut on_event: impl FnMut(WsEvent)) {
    let connector = Connector::Rustls(Arc::new(crate::deps::netrun::tls::client_config()));

    let stream = match connect_async_tls_with_config(&url, None, false, Some(connector)).await {
        Ok((stream, _)) => stream,
        Err(err) => {
            on_event(WsEvent::Error(format!("Failed to connect to {url}: {err}")));
            on_event(WsEvent::Closed);
            return;
        }
    };

    on_event(WsEvent::Opened);

    let (mut write, mut read) = stream.split();

    loop {
        tokio::select! {
            message = read.next() => match message {
                Some(Ok(Message::Text(text))) => on_event(WsEvent::Message(text.to_string())),
                Some(Ok(Message::Binary(bytes))) => on_event(WsEvent::Binary(bytes.to_vec())),
                Some(Ok(Message::Ping(data))) => {
                    if let Err(err) = write.send(Message::Pong(data)).await {
                        error!("Failed to answer a WebSocket ping: {err}");
                    }
                }
                Some(Ok(Message::Close(_))) | None => {
                    on_event(WsEvent::Closed);
                    return;
                }
                Some(Ok(_)) => debug!("WebSocket ignored a frame that is no message"),
                Some(Err(err)) => {
                    on_event(WsEvent::Error(format!("WebSocket failed: {err}")));
                    on_event(WsEvent::Closed);
                    return;
                }
            },
            command = commands.recv() => match command {
                Some(Command::Send(message)) => {
                    if let Err(err) = write.send(Message::text(message)).await {
                        on_event(WsEvent::Error(format!("Failed to send over WebSocket: {err}")));
                        on_event(WsEvent::Closed);
                        return;
                    }
                }
                Some(Command::SendBytes(bytes)) => {
                    if let Err(err) = write.send(Message::binary(bytes)).await {
                        on_event(WsEvent::Error(format!("Failed to send over WebSocket: {err}")));
                        on_event(WsEvent::Closed);
                        return;
                    }
                }
                Some(Command::Close) => {
                    send_close(&mut write).await;
                    on_event(WsEvent::Closed);
                    return;
                }
                // The handle is dropped, nobody listens anymore, so the
                // connection ends without a Closed event.
                None => {
                    send_close(&mut write).await;
                    return;
                }
            },
        }
    }
}

async fn send_close(write: &mut WriteHalf) {
    if let Err(err) = write.send(Message::Close(None)).await {
        debug!("WebSocket close frame not delivered: {err}");
    }
}
