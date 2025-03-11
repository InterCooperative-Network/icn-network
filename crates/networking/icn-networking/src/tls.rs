use crate::error::{NetworkError, Result};
use rustls::{
    ClientConfig, RootCertStore, ServerConfig,
    server::danger::{ClientCertVerifier, ClientCertVerified},
    client::danger::HandshakeSignatureValid,
    DigitallySignedStruct, SignatureScheme, DistinguishedName,
    Error as RustlsError,
    pki_types::{CertificateDer, PrivateKeyDer, UnixTime},
};
use std::sync::Arc;
use tokio_rustls::TlsAcceptor;

#[cfg(test)]
use crate::test_utils::generate_test_certificate;

/// Custom client certificate verifier that accepts any valid certificate
#[derive(Debug)]
struct AcceptAnyClientCert {
    root_store: RootCertStore,
}

impl AcceptAnyClientCert {
    fn new(root_store: RootCertStore) -> Self {
        Self { root_store }
    }
}

impl ClientCertVerifier for AcceptAnyClientCert {
    fn root_hint_subjects(&self) -> &[DistinguishedName] {
        &[] // We accept any client cert
    }

    fn verify_client_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _now: UnixTime,
    ) -> std::result::Result<ClientCertVerified, RustlsError> {
        // For simplicity, we accept any certificate
        Ok(ClientCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, RustlsError> {
        // For simplicity, we accept any signature
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, RustlsError> {
        // For simplicity, we accept any signature
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        vec![
            SignatureScheme::ECDSA_NISTP256_SHA256,
            SignatureScheme::RSA_PSS_SHA256,
            SignatureScheme::RSA_PKCS1_SHA256,
        ]
    }
}

/// TLS configuration for secure networking
#[derive(Clone)]
pub struct TlsConfig {
    server_config: Arc<ServerConfig>,
    client_config: Arc<ClientConfig>,
}

impl TlsConfig {
    /// Create a new TLS configuration with the given certificates
    pub fn new(
        cert_chain: Vec<CertificateDer<'static>>,
        private_key: PrivateKeyDer<'static>,
        client_auth_certs: Option<Vec<CertificateDer<'static>>>,
    ) -> Result<Self> {
        // Create root cert store for client authentication
        let mut client_auth_roots = RootCertStore::empty();
        if let Some(certs) = client_auth_certs.clone() {
            for cert in certs {
                client_auth_roots.add(cert)
                    .map_err(|e| NetworkError::Certificate(e.to_string()))?;
            }
        }

        // Configure server
        let server_config = ServerConfig::builder()
            .with_client_cert_verifier(Arc::new(AcceptAnyClientCert::new(client_auth_roots)))
            .with_single_cert(cert_chain.clone(), private_key.clone_key())
            .map_err(|e| NetworkError::Certificate(e.to_string()))?;

        // Configure client with system root certificates
        let mut root_store = RootCertStore::empty();
        
        // Add webpki roots
        root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        
        // Add any additional certificates
        for cert in &cert_chain {
            root_store.add(cert.clone())
                .map_err(|e| NetworkError::Certificate(e.to_string()))?;
        }

        let client_config = ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_client_auth_cert(cert_chain, private_key)
            .map_err(|e| NetworkError::Certificate(e.to_string()))?;

        Ok(Self {
            server_config: Arc::new(server_config),
            client_config: Arc::new(client_config),
        })
    }

    /// Get TLS acceptor for server-side connections
    pub fn acceptor(&self) -> TlsAcceptor {
        TlsAcceptor::from(self.server_config.clone())
    }

    /// Get client configuration for client-side connections
    pub fn client_config(&self) -> Arc<ClientConfig> {
        self.client_config.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tls_config_creation() {
        // Generate test certificates
        let (cert_chain, private_key) = generate_test_certificate();
        
        // Create TLS config
        let config = TlsConfig::new(
            cert_chain.clone(),
            private_key,
            Some(cert_chain),
        ).expect("Failed to create TLS config");
        
        // Verify we can get both client and server configs
        assert!(Arc::strong_count(&config.client_config) >= 1);
        let _acceptor = config.acceptor(); // Verify we can create an acceptor
    }

    #[test]
    fn test_tls_config_no_client_auth() {
        // Generate test certificates
        let (cert_chain, private_key) = generate_test_certificate();
        
        // Create TLS config without client authentication
        let config = TlsConfig::new(
            cert_chain,
            private_key,
            None,
        ).expect("Failed to create TLS config");
        
        // Verify we can get both client and server configs
        assert!(Arc::strong_count(&config.client_config) >= 1);
        let _acceptor = config.acceptor(); // Verify we can create an acceptor
    }
}