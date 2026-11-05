use std::{env::temp_dir, fs::remove_dir_all, net::SocketAddr, process::id, sync::Arc, time::Duration};

use anyhow::Result;
use axum::{extract::ConnectInfo, routing::get};
use reqwest::Client;
use rustls::{ClientConfig, RootCertStore, crypto::ring::default_provider, pki_types::CertificateDer};
use tokio::{net::TcpStream, sync::oneshot, time::timeout};

use super::*;

/// A client that trusts only the certificate in `dir`, the way a test can
/// without the pinned client of the `hilen` crate.
fn trusting_client(dir: &Path) -> Result<Client> {
    let mut roots = RootCertStore::empty();
    roots.add(CertificateDer::from(fs::read(dir.join(CERT_FILE))?))?;
    let config = ClientConfig::builder_with_provider(Arc::new(default_provider()))
        .with_safe_default_protocol_versions()?
        .with_root_certificates(roots)
        .with_no_client_auth();
    Ok(Client::builder().tls_backend_preconfigured(config).build()?)
}

#[tokio::test]
async fn the_tls_port_serves_the_router_with_headers_and_peer() -> Result<()> {
    let dir = temp_dir().join(format!("hilen-server-tls-{}", id()));
    let cert = LocalCert::load_or_make(&dir)?;
    assert_eq!(cert.fingerprint(), fingerprint(&fs::read(dir.join(CERT_FILE))?));
    assert_eq!(LocalCert::load_or_make(&dir)?.fingerprint(), cert.fingerprint());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(dir.join(KEY_FILE))?.permissions().mode();
        assert_eq!(mode & 0o777, 0o600, "only the owner reads the key");
    }

    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();
    let router = Router::new().route(
        "/peer",
        get(|ConnectInfo(peer): ConnectInfo<SocketAddr>| async move { peer.ip().to_string() }),
    );
    let (stop, stopped) = oneshot::channel::<()>();
    let server = spawn(async move {
        serve_tls_until(listener, &cert, router, async move {
            if stopped.await.is_err() {
                tracing::debug!("the test dropped the stop");
            }
        })
        .await
    });

    // A client that connects and never starts the handshake.
    let stalled = TcpStream::connect(("127.0.0.1", port)).await?;

    let client = trusting_client(&dir)?;
    let answer = timeout(
        Duration::from_secs(5),
        client.get(format!("https://localhost:{port}/peer")).send(),
    )
    .await??;
    assert_eq!(answer.headers()["x-content-type-options"], "nosniff");
    assert_eq!(answer.text().await?, "127.0.0.1");

    drop(stalled);
    stop.send(()).map_err(|()| anyhow::anyhow!("the server is gone"))?;
    timeout(Duration::from_secs(5), server).await???;
    remove_dir_all(&dir)?;
    Ok(())
}
