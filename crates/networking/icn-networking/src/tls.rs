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
#[derive(Debug)]
pub struct TlsConfig {
    /// Certificate path
    pub cert_path: Option<String>,
    
    /// Private key path
    pub key_path: Option<String>,
    
    /// Root CA certificates path
    pub ca_path: Option<String>,
    
    /// Server name override
    pub server_name_override: Option<String>,
    
    /// Certificate chain
    pub cert_chain: Option<Vec<CertificateDer<'static>>>,
    
    /// Private key
    pub private_key: Option<PrivateKeyDer<'static>>,
    
    /// Root CA certificates
    pub root_certs: Option<Vec<CertificateDer<'static>>>,
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            cert_path: None,
            key_path: None,
            ca_path: None,
            server_name_override: None,
            cert_chain: None,
            private_key: None,
            root_certs: None,
        }
    }
}

impl TlsConfig {
    /// Create a new TLS configuration with certificate files
    pub fn new(
        cert_path: impl AsRef<str>,
        key_path: impl AsRef<str>,
        ca_path: Option<impl AsRef<str>>,
    ) -> Result<Self> {
        let mut config = Self::default();
        
        config.cert_path = Some(cert_path.as_ref().to_string());
        config.key_path = Some(key_path.as_ref().to_string());
        
        // Store CA path if provided
        let ca_path_str = ca_path.map(|p| p.as_ref().to_string());
        if let Some(path) = &ca_path_str {
            config.ca_path = Some(path.clone());
        }
        
        // Load certificates and keys
        config.load_certificates()?;
        config.load_private_key()?;
        
        // Load root certificates if provided
        if let Some(path) = &ca_path_str {
            config.load_root_certs(path)?;
        }
        
        Ok(config)
    }
    
    /// Load certificates from file
    pub fn load_certificates(&mut self) -> Result<Vec<CertificateDer<'static>>> {
        if let Some(cert_path) = &self.cert_path {
            let cert_file = File::open(cert_path)
                .map_err(|e| NetworkError::Io(e))?;
            let mut reader = BufReader::new(cert_file);
            let certs = rustls_pemfile::certs(&mut reader)
                .map(|cert_result| cert_result.map_err(|e| NetworkError::Tls(e.to_string())))
                .collect::<std::result::Result<Vec<_>, NetworkError>>()?;
            
            if certs.is_empty() {
                return Err(NetworkError::Tls("No certificates found".to_string()));
            }
            
            self.cert_chain = Some(certs.clone());
            return Ok(certs);
        }
        
        Err(NetworkError::Tls("Certificate path not set".to_string()))
    }
    
    /// Load private key from file
    pub fn load_private_key(&mut self) -> Result<()> {
        if let Some(key_path) = &self.key_path {
            let key_file = File::open(key_path)
                .map_err(|e| NetworkError::Io(e))?;
            let mut reader = BufReader::new(key_file);
            
            // Try to load PKCS8 private keys
            let pkcs8_keys: Vec<_> = rustls_pemfile::pkcs8_private_keys(&mut reader)
                .filter_map(|key_result| key_result.ok())
                .collect();
            
            if !pkcs8_keys.is_empty() {
                self.private_key = Some(PrivateKeyDer::Pkcs8(pkcs8_keys[0].clone_key()));
                return Ok(());
            }
            
            // Reset the reader
            reader = BufReader::new(File::open(key_path).map_err(|e| NetworkError::Io(e))?);
            
            // Try to load RSA private keys
            let rsa_keys: Vec<_> = rustls_pemfile::rsa_private_keys(&mut reader)
                .filter_map(|key_result| key_result.ok())
                .collect();
            
            if !rsa_keys.is_empty() {
                self.private_key = Some(PrivateKeyDer::Pkcs1(rsa_keys[0].clone_key()));
                return Ok(());
            }
            
            return Err(NetworkError::Tls("No private key found".to_string()));
        }
        
        Err(NetworkError::Tls("Private key path not set".to_string()))
    }
    
    /// Load root certificates from file
    pub fn load_root_certs(&mut self, ca_path: &str) -> Result<()> {
        let ca_file = File::open(ca_path)
            .map_err(|e| NetworkError::Io(e))?;
        let mut reader = BufReader::new(ca_file);
        let certs = rustls_pemfile::certs(&mut reader)
            .map(|cert_result| cert_result.map_err(|e| NetworkError::Tls(e.to_string())))
            .collect::<std::result::Result<Vec<_>, NetworkError>>()?;
        
        if certs.is_empty() {
            return Err(NetworkError::Tls("No CA certificates found".to_string()));
        }
        
        self.root_certs = Some(certs);
        Ok(())
    }
    
    /// Create a server configuration
    pub fn server_config(&self) -> Result<ServerConfig> {
        // First check if we have the necessary certificates and keys
        if self.cert_chain.as_ref().map_or(true, |c| c.is_empty()) {
            return Err(NetworkError::Tls("No certificates loaded".to_string()));
        }
        
        if self.private_key.is_none() {
            return Err(NetworkError::Tls("No private key loaded".to_string()));
        }
        
        // We need to get owned versions of the cert chain and private key
        let cert_chain = self.cert_chain.as_ref().unwrap().clone();
        let private_key = match self.private_key.as_ref().unwrap() {
            PrivateKeyDer::Pkcs1(key) => PrivateKeyDer::Pkcs1(key.clone_key()),
            PrivateKeyDer::Pkcs8(key) => PrivateKeyDer::Pkcs8(key.clone_key()),
            PrivateKeyDer::Sec1(key) => PrivateKeyDer::Sec1(key.clone_key()),
            _ => return Err(NetworkError::Tls("Unsupported private key format".to_string())),
        };
        
        // Create a server config
        let server_config = ServerConfig::builder()
            .with_no_client_auth() // For now, we don't require client authentication
            .with_single_cert(cert_chain, private_key)
            .map_err(|e| NetworkError::Tls(format!("Failed to create server config: {}", e)))?;
        
        Ok(server_config)
    }
    
    /// Create a client configuration
    pub fn client_config(&self) -> Result<ClientConfig> {
        let mut root_cert_store = RootCertStore::empty();
        
        // Add our root certificates
        if let Some(certs) = &self.root_certs {
            for cert in certs {
                root_cert_store.add(cert.clone())
                    .map_err(|e| NetworkError::Tls(format!("Failed to add root certificate: {}", e)))?;
            }
        }
        
        // If we don't have any root certificates, we can use the system's root certificates
        // This is more permissive and might not be appropriate for production
        if self.root_certs.is_none() {
            let mut system_roots = RootCertStore::empty();
            for cert in rustls_native_certs::load_native_certs()
                .map_err(|e| NetworkError::Tls(format!("Failed to load system root certificates: {}", e)))? {
                system_roots.add(cert)
                    .map_err(|e| NetworkError::Tls(format!("Failed to add system root certificate: {}", e)))?;
            }
            root_cert_store = system_roots;
        }
        
        // Create a client config
        let mut client_config = ClientConfig::builder()
            .with_root_certificates(root_cert_store.clone())
            .with_no_client_auth();
        
        // If we have a client certificate, use it
        if let (Some(cert_chain), Some(private_key)) = (&self.cert_chain, &self.private_key) {
            let cert_chain = cert_chain.clone();
            let private_key = match private_key {
                PrivateKeyDer::Pkcs1(key) => PrivateKeyDer::Pkcs1(key.clone_key()),
                PrivateKeyDer::Pkcs8(key) => PrivateKeyDer::Pkcs8(key.clone_key()),
                PrivateKeyDer::Sec1(key) => PrivateKeyDer::Sec1(key.clone_key()),
                _ => return Err(NetworkError::Tls("Unsupported private key format".to_string())),
            };
            
            client_config = ClientConfig::builder()
                .with_root_certificates(root_cert_store.clone())
                .with_client_auth_cert(cert_chain, private_key)
                .map_err(|e| NetworkError::Tls(e.to_string()))?;
        }
        
        Ok(client_config)
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
    #[cfg(feature = "testing")]
    pub fn generate_self_signed_cert(common_name: &str) -> Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>)> {
        use rcgen::{Certificate, CertificateParams, DnType, KeyPair, PKCS_ECDSA_P256_SHA256};
        
        let mut params = CertificateParams::new(vec![common_name.to_string()]);
        params.distinguished_name.push(DnType::CommonName, common_name);
        params.alg = &PKCS_ECDSA_P256_SHA256;
        
        let cert = Certificate::from_params(params)
            .map_err(|e| NetworkError::Tls(format!("Failed to generate certificate: {}", e)))?;
        
        let cert_der = cert.serialize_der()
            .map_err(|e| NetworkError::Tls(format!("Failed to serialize certificate: {}", e)))?;
        
        let key_der = cert.serialize_private_key_der();
        
        let cert_chain = vec![CertificateDer::from(cert_der)];
        let private_key = PrivateKeyDer::Pkcs8(key_der.into());
        
        Ok((cert_chain, private_key))
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
        assert!(!config.cert_chain.as_ref().unwrap().is_empty());
        assert!(config.private_key.is_some());
        
        // Clean up
        std::fs::remove_file(cert_path).unwrap();
        std::fs::remove_file(key_path).unwrap();
    }
}