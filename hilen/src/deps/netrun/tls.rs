//! One rustls client config for every native TLS connection here.
//!
//! The provider is named explicitly so this config never depends on the
//! process default being installed first. Ring is the only provider in
//! the dependency graph, aws-lc-sys does not cross-compile for windows
//! in the xwin release container. Bundled webpki roots also skip the JVM
//! certificate path that breaks on android.

use std::sync::Arc;

pub(crate) fn client_config() -> rustls::ClientConfig {
    let roots = rustls::RootCertStore {
        roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
    };

    rustls::ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
        .with_safe_default_protocol_versions()
        .expect("Failed to select TLS protocol versions")
        .with_root_certificates(roots)
        .with_no_client_auth()
}
