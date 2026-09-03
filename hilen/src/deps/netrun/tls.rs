//! One rustls client config for every native TLS connection here.
//!
//! The provider is named explicitly so this config never depends on the
//! process default being installed first. Ring is the only provider in
//! the dependency graph, aws-lc-sys does not cross-compile for windows
//! in the xwin release container. Bundled webpki roots also skip the JVM
//! certificate path that breaks on android.

use std::sync::{Arc, Once};

use log::debug;
#[cfg(test)]
use parking_lot::Mutex;
use rustls::{ClientConfig, RootCertStore, crypto::ring::default_provider, pki_types::CertificateDer};

static PROVIDER: Once = Once::new();

#[cfg(test)]
static TEST_ROOTS: Mutex<Vec<CertificateDer<'static>>> = Mutex::new(Vec::new());

/// Reqwest is built with `rustls-no-provider`, so a client built before
/// the process default is in place panics. Every reqwest entry point here
/// calls this first. A second install is fine, the first one stays.
pub(crate) fn install_provider() {
    PROVIDER.call_once(|| {
        if default_provider().install_default().is_err() {
            debug!("rustls default crypto provider was already installed");
        }
    });
}

pub(crate) fn client_config() -> ClientConfig {
    ClientConfig::builder_with_provider(Arc::new(default_provider()))
        .with_safe_default_protocol_versions()
        .expect("Failed to select TLS protocol versions")
        .with_root_certificates(root_store())
        .with_no_client_auth()
}

fn root_store() -> RootCertStore {
    let mut roots = RootCertStore {
        roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
    };

    for cert in test_roots() {
        roots.add(cert).expect("Failed to trust a test certificate");
    }

    roots
}

/// Trusts a certificate for the rest of the test process, so a test can
/// stand up a TLS server behind a self signed one.
#[cfg(test)]
pub(crate) fn trust_for_tests(cert: CertificateDer<'static>) {
    TEST_ROOTS.lock().push(cert);
}

#[cfg(test)]
fn test_roots() -> Vec<CertificateDer<'static>> {
    TEST_ROOTS.lock().clone()
}

#[cfg(not(test))]
fn test_roots() -> Vec<CertificateDer<'static>> {
    Vec::new()
}
