//! A client that trusts exactly 1 certificate, named by its SHA-256
//! fingerprint, for a backend on the same network that serves its own
//! certificate with `hilen_server::serve_tls`. The fingerprint must come
//! over a connection the app trusts already, like the public address of the
//! same backend.

use std::sync::Arc;

use anyhow::Result;
use reqwest::Client;
use rustls::{
    CertificateError, ClientConfig, DigitallySignedStruct, Error, SignatureScheme,
    client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
    crypto::{CryptoProvider, ring::default_provider, verify_tls12_signature, verify_tls13_signature},
    pki_types::{CertificateDer, ServerName, UnixTime},
};
use sha2::{Digest, Sha256};

use crate::deps::netrun::tls::install_provider;

/// A client for a server whose certificate has the SHA-256 `fingerprint`,
/// in hex. Any other certificate fails the handshake. The name in the URL
/// is not checked, a LAN address is fine.
pub fn pinned_client(fingerprint: &str) -> Result<Client> {
    install_provider();
    let provider = Arc::new(default_provider());
    let verifier = Pinned {
        fingerprint: fingerprint.to_ascii_lowercase(),
        provider:    Arc::clone(&provider),
    };
    let config = ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()?
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(verifier))
        .with_no_client_auth();
    Ok(Client::builder().tls_backend_preconfigured(config).build()?)
}

#[derive(Debug)]
struct Pinned {
    fingerprint: String,
    provider:    Arc<CryptoProvider>,
}

impl ServerCertVerifier for Pinned {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, Error> {
        if hex::encode(Sha256::digest(end_entity)) == self.fingerprint {
            Ok(ServerCertVerified::assertion())
        } else {
            Err(Error::InvalidCertificate(
                CertificateError::ApplicationVerificationFailure,
            ))
        }
    }

    // The handshake still has to prove the server holds the key of the
    // pinned certificate, or a copy of the public certificate would pass.
    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        verify_tls12_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        verify_tls13_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.provider.signature_verification_algorithms.supported_schemes()
    }
}

#[cfg(test)]
mod tests {
    use std::{env::temp_dir, fs::remove_dir_all, process::id};

    use anyhow::Result;
    use axum::{Router, routing::get};
    use hilen_server::{LocalCert, serve_tls};
    use tokio::{net::TcpListener, spawn};

    use super::pinned_client;

    #[tokio::test]
    async fn the_pinned_certificate_is_trusted_and_no_other() -> Result<()> {
        let dir = temp_dir().join(format!("hilen-pinned-{}", id()));
        let cert = LocalCert::load_or_make(&dir)?;
        let again = LocalCert::load_or_make(&dir)?;
        assert_eq!(
            cert.fingerprint(),
            again.fingerprint(),
            "a restart keeps the certificate"
        );

        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let url = format!("https://{}/", listener.local_addr()?);
        let router = Router::new().route("/", get(|| async { "hello" }));
        let fingerprint = cert.fingerprint().to_uppercase();
        let server = spawn(async move { serve_tls(listener, &cert, router).await });

        let answer = pinned_client(&fingerprint)?.get(&url).send().await?.text().await?;
        assert_eq!(answer, "hello");

        let wrong = "00".repeat(32);
        assert!(pinned_client(&wrong)?.get(&url).send().await.is_err());
        assert!(
            reqwest::get(&url).await.is_err(),
            "a client with no pin refuses it"
        );

        server.abort();
        remove_dir_all(&dir)?;
        Ok(())
    }
}
