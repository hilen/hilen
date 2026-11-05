//! A second port of a backend with TLS from a certificate the backend made
//! itself, for clients on the same network.
//!
//! No public authority signs a certificate for a LAN address. So a client
//! gets the SHA-256 fingerprint of this certificate over the public address
//! first, and then trusts this 1 certificate by it, see
//! `hilen::net::pinned_client`.

use std::{
    fs,
    future::pending,
    io,
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use anyhow::{Context, Result};
use axum::{
    Router,
    serve::{Listener, ListenerExt},
};
use rcgen::generate_simple_self_signed;
use rustls::{
    ServerConfig,
    crypto::ring::default_provider,
    pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer},
};
use sha2::{Digest, Sha256};
use tokio::{
    net::{TcpListener, TcpStream},
    select, spawn,
    sync::mpsc::{Receiver, Sender, channel},
    time::{sleep, timeout},
};
use tokio_rustls::{TlsAcceptor, server::TlsStream};

use crate::helpers::{SHUTDOWN_LIMIT, serve_until, stop_signal};

const CERT_FILE: &str = "local-tls.cert.der";
const KEY_FILE: &str = "local-tls.key.der";

/// A client that opens a connection and sends nothing must not hold a slot
/// for long.
const HANDSHAKE_LIMIT: Duration = Duration::from_secs(10);

/// The connections whose handshake is done and that wait for the server.
const READY_QUEUE: usize = 64;

/// The self-signed certificate of a backend, kept in a folder so its
/// fingerprint stays the same across restarts.
pub struct LocalCert {
    config:      Arc<ServerConfig>,
    fingerprint: String,
}

impl LocalCert {
    /// Reads the certificate kept in `dir`, or makes one and keeps it there.
    pub fn load_or_make(dir: &Path) -> Result<Self> {
        let cert_file = dir.join(CERT_FILE);
        let key_file = dir.join(KEY_FILE);
        let (cert, key) = if cert_file.exists() && key_file.exists() {
            let cert = read(&cert_file)?;
            let key = read(&key_file)?;
            tracing::info!("local TLS certificate read from {}", cert_file.display());
            (cert, key)
        } else {
            let made = generate_simple_self_signed(vec!["localhost".to_string()])
                .context("make a local TLS certificate")?;
            let cert = made.cert.der().to_vec();
            let key = made.signing_key.serialize_der();
            fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
            write_private(&key_file, &key)?;
            fs::write(&cert_file, &cert).with_context(|| format!("write {}", cert_file.display()))?;
            tracing::info!("local TLS certificate made in {}", cert_file.display());
            (cert, key)
        };
        Self::from_der(cert, key)
    }

    fn from_der(cert: Vec<u8>, key: Vec<u8>) -> Result<Self> {
        let fingerprint = fingerprint(&cert);
        let config = ServerConfig::builder_with_provider(Arc::new(default_provider()))
            .with_safe_default_protocol_versions()?
            .with_no_client_auth()
            .with_single_cert(
                vec![CertificateDer::from(cert)],
                PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key)),
            )
            .context("the local TLS certificate does not load")?;
        Ok(Self {
            config: Arc::new(config),
            fingerprint,
        })
    }

    /// The SHA-256 of the certificate as lowercase hex, what a client
    /// compares the certificate of the server with.
    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }
}

/// The SHA-256 of a certificate in DER as lowercase hex.
pub fn fingerprint(cert_der: &[u8]) -> String {
    hex::encode(Sha256::digest(cert_der))
}

fn read(file: &PathBuf) -> Result<Vec<u8>> {
    fs::read(file).with_context(|| format!("read {}", file.display()))
}

/// Only the owner may read the key.
fn write_private(file: &Path, bytes: &[u8]) -> Result<()> {
    #[cfg(unix)]
    {
        use std::{io::Write, os::unix::fs::OpenOptionsExt};
        let mut out = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(file)
            .with_context(|| format!("create {}", file.display()))?;
        out.write_all(bytes).with_context(|| format!("write {}", file.display()))
    }
    #[cfg(not(unix))]
    {
        fs::write(file, bytes).with_context(|| format!("write {}", file.display()))
    }
}

/// Serves `router` with TLS on `listener`, with the same security headers,
/// peer address and stop as [`serve_listener`](crate::serve_listener).
pub async fn serve_tls(listener: TcpListener, cert: &LocalCert, router: Router) -> Result<()> {
    serve_tls_until(listener, cert, router, stop_signal()).await
}

async fn serve_tls_until(
    listener: TcpListener,
    cert: &LocalCert,
    router: Router,
    stop: impl Future<Output = ()> + Send + 'static,
) -> Result<()> {
    let local = listener.local_addr()?;
    tracing::info!("listening on {local} with TLS, certificate {}", cert.fingerprint);
    let (ready, waiting) = channel(READY_QUEUE);
    spawn(accept_loop(
        listener,
        TlsAcceptor::from(Arc::clone(&cert.config)),
        ready,
    ));
    let listener = TlsListener { waiting, local }.tap_io(|_| {});
    serve_until(listener, router, stop, SHUTDOWN_LIMIT).await
}

/// Takes the connections and runs each handshake in its own task, so a slow
/// client does not hold the next one. Ends when the server is gone.
async fn accept_loop(
    listener: TcpListener,
    acceptor: TlsAcceptor,
    ready: Sender<(TlsStream<TcpStream>, SocketAddr)>,
) {
    loop {
        let accepted = select! {
            () = ready.closed() => return,
            accepted = listener.accept() => accepted,
        };
        let (stream, peer) = match accepted {
            Ok(accepted) => accepted,
            Err(error) => {
                // Like axum: an error here is mostly too many open files,
                // which a short wait lets pass.
                tracing::warn!("TLS port accept failed: {error}");
                sleep(Duration::from_secs(1)).await;
                continue;
            }
        };
        let acceptor = acceptor.clone();
        let ready = ready.clone();
        spawn(async move {
            match timeout(HANDSHAKE_LIMIT, acceptor.accept(stream)).await {
                Ok(Ok(stream)) => {
                    if ready.send((stream, peer)).await.is_err() {
                        tracing::debug!("TLS connection of {peer} came after the server stopped");
                    }
                }
                Ok(Err(error)) => tracing::debug!("TLS handshake of {peer} failed: {error}"),
                Err(_) => tracing::debug!("TLS handshake of {peer} took over {HANDSHAKE_LIMIT:?}"),
            }
        });
    }
}

struct TlsListener {
    waiting: Receiver<(TlsStream<TcpStream>, SocketAddr)>,
    local:   SocketAddr,
}

impl Listener for TlsListener {
    type Io = TlsStream<TcpStream>;
    type Addr = SocketAddr;

    async fn accept(&mut self) -> (Self::Io, Self::Addr) {
        match self.waiting.recv().await {
            Some(connection) => connection,
            // The accept loop ends only when this listener is gone.
            None => pending().await,
        }
    }

    fn local_addr(&self) -> io::Result<Self::Addr> {
        Ok(self.local)
    }
}

#[cfg(test)]
mod tests;
