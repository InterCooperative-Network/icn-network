use crate::error::{NetworkError, Result};
use rustls::{
    ClientConfig, RootCertStore, ServerConfig,
    server::AllowAnyAuthenticatedClient,
    Certificate, PrivateKey,
};
use std::sync::Arc;
use tokio_rustls::TlsAcceptor;

/// TLS configuration for secure networking
pub struct TlsConfig {
    server_config: Arc<ServerConfig>,
    client_config: Arc<ClientConfig>,
}

impl TlsConfig {
    /// Create a new TLS configuration with the given certificates
    pub fn new(
        cert_chain: Vec<Certificate>,
        private_key: PrivateKey,
        client_auth_certs: Option<Vec<Certificate>>,
    ) -> Result<Self> {
        // Create root cert store for client authentication
        let mut client_auth_roots = RootCertStore::empty();
        if let Some(certs) = client_auth_certs {
            for cert in certs {
                client_auth_roots.add(&cert)
                    .map_err(|e| NetworkError::Certificate(e.to_string()))?;
            }
        }

        // Configure server
        let server_config = ServerConfig::builder()
            .with_safe_defaults()
            .with_client_cert_verifier(Arc::new(AllowAnyAuthenticatedClient::new(client_auth_roots)))
            .with_single_cert(cert_chain.clone(), private_key.clone())
            .map_err(|e| NetworkError::Certificate(e.to_string()))?;

        // Configure client with system root certificates
        let mut root_store = RootCertStore::empty();
        root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        
        // Add any additional certificates
        for cert in cert_chain {
            root_store.add(&cert)
                .map_err(|e| NetworkError::Certificate(e.to_string()))?;
        }

        let client_config = ClientConfig::builder()
            .with_safe_defaults()
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
    use rustls::Certificate;
    use std::fs;

    #[test]
    fn test_tls_config_creation() {
        // This test would load test certificates and create a TLS config
        // In a real implementation, we would:
        // 1. Load test certificates from files or generate them
        // 2. Create TLS config
        // 3. Verify both client and server configs are created
    }
}