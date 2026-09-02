use reqwest::Client;

#[cfg(android)]
use crate::deps::netrun::tls::client_config;
#[cfg(not_wasm)]
use crate::deps::netrun::tls::install_provider;

/// A client whose TLS verifier works on every platform.
///
/// Reqwest's rustls setup verifies certificates through the OS. On android
/// that path calls into the JVM and needs a Kotlin component the app does not
/// carry, so the first request panics inside rustls-platform-verifier.
/// The shared config with bundled webpki roots skips the JVM entirely.
#[cfg(android)]
pub(crate) fn client() -> Client {
    install_provider();
    Client::builder()
        .use_preconfigured_tls(client_config())
        .build()
        .expect("Failed to build the android TLS client")
}

#[cfg(not(android))]
pub(crate) fn client() -> Client {
    #[cfg(not_wasm)]
    install_provider();
    Client::new()
}
