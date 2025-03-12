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
use rustls_pemfile::certs;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

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

/// TLS configuration
#[derive(Debug, Clone)]
pub struct TlsConfig {
    /// Certificate path
    pub cert_path: Option<PathBuf>,
    
    /// Private key path
    pub key_path: Option<PathBuf>,
    
    /// Root CA certificates path
    pub ca_path: Option<PathBuf>,
    
    /// Server name override
    pub server_name_override: Option<String>,
    
    /// Certificate chain
    pub cert_chain: Vec<CertificateDer<'static>>,
    
    /// Private key
    pub private_key: Option<PrivateKeyDer<'static>>,
    
    /// Root CA certificates
    pub root_certs: Vec<CertificateDer<'static>>,
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            cert_path: None,
            key_path: None,
            ca_path: None,
            server_name_override: None,
            cert_chain: Vec::new(),
            private_key: None,
            root_certs: Vec::new(),
        }
    }
}

impl TlsConfig {
    /// Create a new TLS configuration with certificate files
    pub fn new(
        cert_path: impl AsRef<Path>,
        key_path: impl AsRef<Path>,
        ca_path: Option<impl AsRef<Path>>,
    ) -> Result<Self> {
        let mut config = Self::default();
        
        config.cert_path = Some(cert_path.as_ref().to_path_buf());
        config.key_path = Some(key_path.as_ref().to_path_buf());
        
        if let Some(ca_path) = ca_path {
            config.ca_path = Some(ca_path.as_ref().to_path_buf());
        }
        
        config.load_certificates()?;
        config.load_private_key()?;
        
        if let Some(ca_path) = &config.ca_path {
            config.load_root_certs(ca_path)?;
        }
        
        Ok(config)
    }
    
    /// Load certificates from file
    pub fn load_certificates(&mut self) -> Result<()> {
        if let Some(cert_path) = &self.cert_path {
            let cert_file = File::open(cert_path)
                .map_err(|e| NetworkError::Io(e))?;
            let mut reader = BufReader::new(cert_file);
            self.cert_chain = certs(&mut reader)
                .map(|cert_result| cert_result.map_err(|e| NetworkError::Tls(e.to_string())))
                .collect::<Result<Vec<_>>>()?;
            
            if self.cert_chain.is_empty() {
                return Err(NetworkError::Tls("No certificates found".to_string()));
            }
        }
        
        Ok(())
    }
    
    /// Load private key from file
    pub fn load_private_key(&mut self) -> Result<()> {
        if let Some(key_path) = &self.key_path {
            let key_file = File::open(key_path)
                .map_err(|e| NetworkError::Io(e))?;
            let mut reader = BufReader::new(key_file);
            
            // Try to read PKCS8 or RSA private keys
            if let Ok(keys) = rustls_pemfile::pkcs8_private_keys(&mut reader) {
                if !keys.is_empty() {
                    self.private_key = Some(PrivateKeyDer::Pkcs8(keys[0].clone()));
                    return Ok(());
                }
            }
            
            // Rewind the reader
            reader = BufReader::new(File::open(key_path).map_err(|e| NetworkError::Io(e))?);
            
            if let Ok(keys) = rustls_pemfile::rsa_private_keys(&mut reader) {
                if !keys.is_empty() {
                    self.private_key = Some(PrivateKeyDer::Pkcs1(keys[0].clone()));
                    return Ok(());
                }
            }
            
            // No keys found
            return Err(NetworkError::Tls("No private key found".to_string()));
        }
        
        Ok(())
    }
    
    /// Load root certificates from file
    pub fn load_root_certs(&mut self, ca_path: impl AsRef<Path>) -> Result<()> {
        let ca_file = File::open(ca_path)
            .map_err(|e| NetworkError::Io(e))?;
        let mut reader = BufReader::new(ca_file);
        let certs = certs(&mut reader)
            .map(|cert_result| cert_result.map_err(|e| NetworkError::Tls(e.to_string())))
            .collect::<Result<Vec<_>>>()?;
        
        if certs.is_empty() {
            return Err(NetworkError::Tls("No CA certificates found".to_string()));
        }
        
        self.root_certs = certs;
        Ok(())
    }
    
    /// Generate a server configuration
    pub fn server_config(&self) -> Result<ServerConfig> {
        // First check if we have the necessary certificates and keys
        if self.cert_chain.is_empty() {
            return Err(NetworkError::Tls("No certificates loaded".to_string()));
        }
        
        if self.private_key.is_none() {
            return Err(NetworkError::Tls("No private key loaded".to_string()));
        }
        
        // Build the server configuration
        let server_config = ServerConfig::builder()
            .with_no_client_auth() // For now, we don't require client authentication
            .with_single_cert(
                self.cert_chain.clone(), 
                self.private_key.clone().unwrap()
            )
            .map_err(|e| NetworkError::Tls(e.to_string()))?;
        
        Ok(server_config)
    }
    
    /// Generate a client configuration
    pub fn client_config(&self) -> Result<ClientConfig> {
        // Build a root certificate store
        let mut root_cert_store = rustls::RootCertStore::empty();
        
        // Add our root certificates
        for cert in &self.root_certs {
            root_cert_store.add(cert.clone())
                .map_err(|e| NetworkError::Tls(format!("Failed to add root certificate: {}", e)))?;
        }
        
        // If we don't have any root certificates, we can use the system's root certificates
        // This is more permissive and might not be appropriate for production
        if self.root_certs.is_empty() {
            let mut system_roots = rustls::RootCertStore::empty();
            for cert in rustls_native_certs::load_native_certs()
                .map_err(|e| NetworkError::Tls(format!("Failed to load system root certificates: {}", e)))? {
                system_roots.add(cert).map_err(|e| NetworkError::Tls(e.to_string()))?;
            }
            
            root_cert_store = system_roots;
        }
        
        // Build the client configuration
        let builder = ClientConfig::builder()
            .with_root_certificates(root_cert_store);
        
        // If we have a client certificate, use it
        if !self.cert_chain.is_empty() && self.private_key.is_some() {
            let client_config = builder.with_client_auth_cert(
                self.cert_chain.clone(),
                self.private_key.clone().unwrap(),
            ).map_err(|e| NetworkError::Tls(e.to_string()))?;
            
            Ok(client_config)
        } else {
            // Otherwise, just use the root certificates
            Ok(builder.with_no_client_auth())
        }
    }
    
    /// Create a TLS connector for client connections
    pub fn connector(&self) -> Result<tokio_rustls::TlsConnector> {
        let client_config = self.client_config()?;
        Ok(tokio_rustls::TlsConnector::from(Arc::new(client_config)))
    }
    
    /// Create a TLS acceptor for server connections
    pub fn acceptor(&self) -> Result<tokio_rustls::TlsAcceptor> {
        let server_config = self.server_config()?;
        Ok(tokio_rustls::TlsAcceptor::from(Arc::new(server_config)))
    }
    
    /// Generate a self-signed certificate for testing
    pub fn generate_self_signed(
        common_name: &str,
        cert_path: impl AsRef<Path>,
        key_path: impl AsRef<Path>,
    ) -> Result<Self> {
        // Generate a certificate and key
        let cert = rcgen::generate_simple_self_signed(vec![common_name.to_string()])
            .map_err(|e| NetworkError::Tls(format!("Failed to generate certificate: {}", e)))?;
        
        // Write the certificate to a file
        std::fs::write(cert_path.as_ref(), cert.serialize_pem()
            .map_err(|e| NetworkError::Tls(format!("Failed to serialize certificate: {}", e)))?)
            .map_err(|e| NetworkError::Io(e))?;
        
        // Write the private key to a file
        std::fs::write(key_path.as_ref(), cert.serialize_private_key_pem())
            .map_err(|e| NetworkError::Io(e))?;
        
        // Load the TLS configuration
        Self::new(cert_path, key_path, None::<&Path>)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    
    #[test]
    fn test_generate_self_signed() {
        let temp_dir = env::temp_dir();
        let cert_path = temp_dir.join("test-cert.pem");
        let key_path = temp_dir.join("test-key.pem");
        
        // Generate a self-signed certificate
        let config = TlsConfig::generate_self_signed(
            "localhost",
            &cert_path,
            &key_path,
        ).unwrap();
        
        // Verify that the certificate and key files exist
        assert!(cert_path.exists());
        assert!(key_path.exists());
        
        // Verify that the configuration has the certificate and key
        assert!(!config.cert_chain.is_empty());
        assert!(config.private_key.is_some());
        
        // Clean up
        std::fs::remove_file(cert_path).unwrap();
        std::fs::remove_file(key_path).unwrap();
    }
}