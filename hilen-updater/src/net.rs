//! The HTTP side, on a client of its own with the same TLS setup as the
//! engine, rustls on the ring provider.

use std::sync::{LazyLock, Once};

use anyhow::{Result, bail};
use log::debug;
use reqwest::Client;
use rustls::crypto::ring::default_provider;

/// Reqwest is built with `rustls-no-provider`, so a client built before the
/// process default is in place panics. A second install is fine, the first
/// one stays, so an app whose engine installed it already loses nothing.
fn install_provider() {
    static PROVIDER: Once = Once::new();

    PROVIDER.call_once(|| {
        if default_provider().install_default().is_err() {
            debug!("rustls default crypto provider was already installed");
        }
    });
}

fn client() -> &'static Client {
    static CLIENT: LazyLock<Client> = LazyLock::new(|| {
        install_provider();
        Client::new()
    });
    &CLIENT
}

/// The body of a successful answer as text.
pub(crate) async fn get_text(url: &str) -> Result<String> {
    let response = client().get(url).send().await?;
    let status = response.status();
    let body = response.text().await?;
    debug!("GET {url}: {status}");

    if !status.is_success() {
        bail!("[{status}] {url}: {body}");
    }

    Ok(body)
}

/// Downloads in chunks and reports the bytes so far plus the total from
/// the Content-Length header, `None` when the server sent no length.
pub(crate) async fn download_with_progress(
    url: &str,
    mut on_progress: impl FnMut(u64, Option<u64>),
) -> Result<Vec<u8>> {
    let mut response = client().get(url).send().await?;
    let status = response.status();

    // Without this a block page or an error page downloads as if it were the
    // file, and the caller only sees a body of the wrong size.
    if !status.is_success() {
        bail!("[{status}] Failed to download {url}");
    }

    let total = response.content_length();
    let mut bytes = Vec::with_capacity(usize::try_from(total.unwrap_or_default()).unwrap_or_default());
    on_progress(0, total);

    while let Some(chunk) = response.chunk().await? {
        bytes.extend_from_slice(&chunk);
        on_progress(bytes.len() as u64, total);
    }

    Ok(bytes)
}
