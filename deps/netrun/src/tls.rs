//! One rustls client config for every native TLS connection here.
//!
//! The provider is named explicitly. The plain builder takes the process
//! default, and with both ring and aws-lc-rs in the dependency graph there
//! is none, which is a panic at connect time. Bundled webpki roots also
//! skip the JVM certificate path that breaks on android.

use std::sync::Arc;

pub(crate) fn client_config() -> rustls::ClientConfig {
    let roots = rustls::RootCertStore {
        roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
    };

    rustls::ClientConfig::builder_with_provider(Arc::new(rustls::crypto::aws_lc_rs::default_provider()))
        .with_safe_default_protocol_versions()
        .expect("Failed to select TLS protocol versions")
        .with_root_certificates(roots)
        .with_no_client_auth()
}
