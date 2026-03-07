use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::ServerConfig;
use tokio_rustls::{TlsAcceptor, TlsConnector};

/// Ensure a self-signed cert+key pair exist in data_dir; generate if missing.
pub fn ensure_cert(data_dir: &PathBuf) -> Result<(CertificateDer<'static>, PrivateKeyDer<'static>)> {
    let cert_path = data_dir.join("cert.pem");
    let key_path = data_dir.join("key.pem");

    if cert_path.exists() && key_path.exists() {
        let cert_pem = std::fs::read(&cert_path).context("read cert.pem")?;
        let key_pem = std::fs::read(&key_path).context("read key.pem")?;

        let cert = rustls_pemfile::certs(&mut cert_pem.as_slice())
            .next()
            .context("no cert in pem")?
            .context("cert parse error")?
            .into_owned();

        let key = rustls_pemfile::private_key(&mut key_pem.as_slice())
            .context("private_key read")?
            .context("no private key in pem")?
            .clone_key();

        return Ok((cert, key));
    }

    // Generate self-signed cert
    let cert_obj = rcgen::generate_simple_self_signed(vec!["InputSync".to_string()])
        .context("rcgen generate")?;

    let cert_pem = cert_obj.cert.pem();
    let key_pem = cert_obj.key_pair.serialize_pem();

    std::fs::create_dir_all(data_dir).context("create data_dir")?;
    std::fs::write(&cert_path, &cert_pem).context("write cert.pem")?;
    std::fs::write(&key_path, &key_pem).context("write key.pem")?;

    let cert = rustls_pemfile::certs(&mut cert_pem.as_bytes())
        .next()
        .context("no cert in generated pem")?
        .context("cert parse")?
        .into_owned();

    let key = rustls_pemfile::private_key(&mut key_pem.as_bytes())
        .context("private_key parse")?
        .context("no key in generated pem")?
        .clone_key();

    tracing::info!("Generated new self-signed TLS certificate in {:?}", data_dir);
    Ok((cert, key))
}

/// Build a TlsAcceptor for the server using the cert in data_dir.
pub fn make_tls_acceptor(data_dir: &PathBuf) -> Result<TlsAcceptor> {
    let (cert, key) = ensure_cert(data_dir)?;

    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![cert], key)
        .context("TLS server config")?;

    Ok(TlsAcceptor::from(Arc::new(config)))
}

/// Build a TlsConnector that accepts any server certificate (TOFU model).
pub fn make_tls_connector() -> TlsConnector {
    let config = rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(AcceptAnyCert))
        .with_no_client_auth();

    TlsConnector::from(Arc::new(config))
}

/// Custom verifier: accept any server cert (log its fingerprint).
#[derive(Debug)]
struct AcceptAnyCert;

impl rustls::client::danger::ServerCertVerifier for AcceptAnyCert {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        use sha2::Digest;
        let digest = sha2::Sha256::digest(end_entity.as_ref());
        let hex: String = digest
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(":");
        tracing::info!("TLS server cert fingerprint (SHA-256): {}", hex);
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        rustls::crypto::ring::default_provider()
            .signature_verification_algorithms
            .supported_schemes()
    }
}
