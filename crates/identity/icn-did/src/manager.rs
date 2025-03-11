//! DID manager implementation
use async_trait::async_trait;
use icn_common::{Error, Result};
use icn_crypto::{KeyPair, KeyType, PublicKey};
use icn_storage_system::StorageOptions;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::{
    DidDocument, VerificationMethod, VerificationMethodReference, PublicKeyMaterial,
    resolver::{DidResolver, IcnDidResolver},
    verification::{AuthenticationChallenge, AuthenticationResponse, VerificationResult},
    DID_METHOD,
};

/// DID manager configuration
#[derive(Debug, Clone)]
pub struct DidManagerConfig {
    /// Storage options for the DID resolver
    pub storage_options: StorageOptions,
    
    /// Default key type for new DIDs
    pub default_key_type: KeyType,
    
    /// Default challenge TTL in seconds
    pub challenge_ttl_seconds: u64,
}

impl Default for DidManagerConfig {
    fn default() -> Self {
        Self {
            storage_options: StorageOptions::default(),
            default_key_type: KeyType::Ed25519,
            challenge_ttl_seconds: 300, // 5 minutes
        }
    }
}

/// DID creation options
#[derive(Debug, Clone)]
pub struct CreateDidOptions {
    /// Key type to use (defaults to manager's default)
    pub key_type: Option<KeyType>,
    
    /// Additional verification methods to add
    pub additional_verification_methods: Vec<VerificationMethod>,
    
    /// Additional service endpoints to add
    pub additional_services: Vec<crate::Service>,
}

impl Default for CreateDidOptions {
    fn default() -> Self {
        Self {
            key_type: None,
            additional_verification_methods: Vec::new(),
            additional_services: Vec::new(),
        }
    }
}

/// DID manager for coordinating DID operations
pub struct DidManager {
    /// The DID resolver
    resolver: Arc<IcnDidResolver>,
    
    /// Configuration options
    config: DidManagerConfig,
}

impl DidManager {
    /// Create a new DID manager
    pub async fn new(config: DidManagerConfig) -> Result<Self> {
        let resolver = IcnDidResolver::new(config.storage_options.clone()).await?;
        
        Ok(Self {
            resolver: Arc::new(resolver),
            config,
        })
    }
    
    /// Create a new DID with the given options
    pub async fn create_did(&self, options: CreateDidOptions) -> Result<(DidDocument, Box<dyn KeyPair>)> {
        // Generate key pair
        let key_type = options.key_type.unwrap_or(self.config.default_key_type);
        let key_pair = icn_crypto::generate_keypair()?;
        
        // Generate a unique identifier
        let id = generate_did_identifier();
        let did = format!("did:{}:{}", DID_METHOD, id);
        
        // Create DID document
        let mut document = DidDocument::new(&id)?;
        
        // Add authentication verification method
        let auth_method = create_verification_method(
            &did,
            "key-1",
            key_type,
            key_pair.public_key(),
        )?;
        
        document.add_verification_method(auth_method.clone());
        document.add_authentication(VerificationMethodReference::Embedded(auth_method));
        
        // Add additional verification methods
        for method in options.additional_verification_methods {
            document.add_verification_method(method.clone());
        }
        
        // Add additional services
        for service in options.additional_services {
            document.add_service(service);
        }
        
        // Store the DID document
        self.resolver.store(&did, document.clone()).await?;
        
        Ok((document, key_pair))
    }
    
    /// Update a DID document
    pub async fn update_did(&self, did: &str, document: DidDocument) -> Result<()> {
        // Validate the document
        if document.id != did {
            return Err(Error::validation("Document ID does not match DID"));
        }
        
        self.resolver.update(did, document).await
    }
    
    /// Deactivate a DID
    pub async fn deactivate_did(&self, did: &str) -> Result<()> {
        self.resolver.deactivate(did).await
    }
    
    /// List all DIDs
    pub async fn list_dids(&self) -> Result<Vec<String>> {
        self.resolver.list_dids().await
    }
    
    /// Get the resolver
    pub fn resolver(&self) -> Arc<IcnDidResolver> {
        self.resolver.clone()
    }

    /// Create an authentication challenge for a DID
    pub async fn create_authentication_challenge(
        &self,
        did: &str,
        verification_method: Option<&str>,
        ttl_secs: Option<u64>,
    ) -> Result<AuthenticationChallenge> {
        let doc = self.resolve_did(did).await?;
        
        // If no verification method specified, use the first authentication method
        let method_id = match verification_method {
            Some(id) => id.to_string(),
            None => doc.authentication.first()
                .ok_or_else(|| Error::validation("No authentication methods available"))?
                .id()
                .to_string(),
        };

        AuthenticationChallenge::new(
            did,
            &method_id,
            ttl_secs.unwrap_or(300), // Default 5 minute TTL
        )
    }

    /// Verify an authentication response
    pub async fn verify_authentication(
        &self,
        response: &AuthenticationResponse,
    ) -> Result<bool> {
        // Check if challenge has expired
        if response.challenge.is_expired()? {
            return Ok(false);
        }

        // Resolve the DID document
        let doc = self.resolve_did(&response.challenge.did).await?;
        
        // Get the verification method
        let method = doc.get_verification_method(&response.challenge.verification_method)
            .ok_or_else(|| Error::not_found("Verification method not found"))?;

        // Verify the signature
        let message = response.challenge.get_message();
        method.verify_signature(&message, &response.signature)
    }

    /// Verify a signature using a specific verification method
    pub async fn verify_signature(
        &self,
        did: &str,
        method_id: &str,
        message: &[u8],
        signature: &icn_crypto::Signature,
    ) -> Result<bool> {
        // Resolve DID document
        let resolution = self.resolve(did).await?;
        let document = resolution.did_document
            .ok_or_else(|| Error::not_found("DID not found"))?;
            
        // Verify signature
        document.verify_signature(method_id, message, signature)
    }

    pub async fn create_authentication_challenge(&self, did: &str, method_id: &str) -> Result<AuthenticationChallenge> {
        // Verify DID exists and method is valid
        let doc = self.resolver.resolve(did).await?;
        
        if !doc.verification_methods.iter().any(|m| m.id == method_id) {
            return Err(Error::not_found(format!("Verification method {} not found", method_id)));
        }

        Ok(AuthenticationChallenge::new(method_id.to_string()))
    }

    pub async fn verify_authentication(&self, did: &str, response: AuthenticationResponse) -> Result<VerificationResult> {
        let doc = self.resolver.resolve(did).await?;
        
        // Find the verification method
        let method = doc.verification_methods.iter()
            .find(|m| m.id == response.challenge.method_id)
            .ok_or_else(|| Error::not_found("Verification method not found"))?;

        // Verify the signature
        let is_valid = response.challenge.verify_signature(
            &method.public_key_bytes()?,
            &response.signature
        )?;

        Ok(VerificationResult {
            is_valid,
            method_id: method.id.clone(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        })
    }

    pub async fn verify_signature(&self, did: &str, method_id: &str, message: &[u8], signature: &[u8]) -> Result<bool> {
        let doc = self.resolver.resolve(did).await?;
        
        // Find the verification method
        let method = doc.verification_methods.iter()
            .find(|m| m.id == method_id)
            .ok_or_else(|| Error::not_found("Verification method not found"))?;

        // Convert signature to proper type
        let sig = Signature::from_bytes(signature)
            .map_err(|e| Error::validation(format!("Invalid signature: {}", e)))?;

        // Verify using the method's public key
        method.verify_signature(message, &sig)
    }
}

#[async_trait]
impl DidResolver for DidManager {
    async fn resolve(&self, did: &str) -> Result<crate::resolver::ResolutionResult> {
        self.resolver.resolve(did).await
    }
    
    fn supports_method(&self, method: &str) -> bool {
        self.resolver.supports_method(method)
    }
}

/// Create a verification method from a public key
fn create_verification_method(
    controller: &str,
    fragment: &str,
    key_type: KeyType,
    public_key: &dyn PublicKey,
) -> Result<VerificationMethod> {
    let method_type = match key_type {
        KeyType::Ed25519 => "Ed25519VerificationKey2020",
        KeyType::Secp256k1 => "EcdsaSecp256k1VerificationKey2019",
        _ => return Err(Error::validation("Unsupported key type for verification method")),
    };
    
    Ok(VerificationMethod {
        id: format!("{}#{}", controller, fragment),
        type_: method_type.to_string(),
        controller: controller.to_string(),
        public_key: PublicKeyMaterial::Ed25519VerificationKey2020(
            public_key.to_base58(),
        ),
    })
}

/// Generate a unique identifier for a DID
fn generate_did_identifier() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let bytes: [u8; 16] = rng.gen();
    hex::encode(bytes)
}

// Helper trait for getting IDs from verification method references
trait VerificationMethodReferenceExt {
    fn get_id(&self) -> &str;
}

impl VerificationMethodReferenceExt for VerificationMethodReference {
    fn get_id(&self) -> &str {
        match self {
            Self::Reference(id) => id,
            Self::Embedded(method) => &method.id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    async fn create_test_manager() -> (DidManager, tempfile::TempDir) {
        let temp_dir = tempdir().unwrap();
        let config = DidManagerConfig {
            storage_options: StorageOptions {
                base_dir: temp_dir.path().to_path_buf(),
                sync_writes: true,
                compress: false,
            },
            ..Default::default()
        };
        
        let manager = DidManager::new(config).await.unwrap();
        (manager, temp_dir)
    }
    
    #[tokio::test]
    async fn test_did_lifecycle() {
        let (manager, _temp) = create_test_manager().await;
        
        // Create a DID
        let (document, key_pair) = manager.create_did(CreateDidOptions::default()).await.unwrap();
        let did = document.id.clone();
        
        // Verify the document can be resolved
        let resolution = manager.resolve(&did).await.unwrap();
        assert!(resolution.did_document.is_some());
        assert_eq!(resolution.did_document.unwrap().id, did);
        
        // Update the document
        let mut updated_doc = document.clone();
        updated_doc.add_service(crate::Service {
            id: format!("{}#service-1", did),
            type_: "TestService".to_string(),
            service_endpoint: "https://example.com".to_string(),
        });
        
        manager.update_did(&did, updated_doc.clone()).await.unwrap();
        
        // Verify the update
        let resolution = manager.resolve(&did).await.unwrap();
        assert!(resolution.did_document.is_some());
        assert_eq!(resolution.did_document.unwrap().service.len(), 1);
        
        // Deactivate the DID
        manager.deactivate_did(&did).await.unwrap();
        
        // Verify deactivation
        let resolution = manager.resolve(&did).await.unwrap();
        assert!(resolution.document_metadata.deactivated.unwrap());
        
        // List DIDs
        let dids = manager.list_dids().await.unwrap();
        assert!(dids.contains(&did));
    }
    
    #[tokio::test]
    async fn test_create_did_with_options() {
        let (manager, _temp) = create_test_manager().await;
        
        // Create a DID with additional options
        let options = CreateDidOptions {
            key_type: Some(KeyType::Ed25519),
            additional_verification_methods: vec![],
            additional_services: vec![
                crate::Service {
                    id: "service-1".to_string(),
                    type_: "TestService".to_string(),
                    service_endpoint: "https://example.com".to_string(),
                },
            ],
        };
        
        let (document, _) = manager.create_did(options).await.unwrap();
        
        // Verify the document
        assert_eq!(document.service.len(), 1);
        assert_eq!(document.service[0].type_, "TestService");
    }
    
    #[tokio::test]
    async fn test_authentication_challenge() {
        let (manager, _temp) = create_test_manager().await;
        
        // Create a DID
        let (doc, _) = manager.create_did(CreateDidOptions::default()).await.unwrap();
        
        // Create challenge
        let challenge = manager.create_authentication_challenge(
            &doc.id,
            Some("#key-1"),
            None
        ).await.unwrap();
        
        assert_eq!(challenge.did, doc.id);
        assert!(!challenge.is_expired().unwrap());
        
        // Test with non-existent DID
        let result = manager.create_authentication_challenge(
            "did:icn:nonexistent",
            None,
            None
        ).await;
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_signature_verification() {
        let (manager, _temp) = create_test_manager().await;
        
        // Create a DID with key pair
        let (doc, key_pair) = manager.create_did(CreateDidOptions::default()).await.unwrap();
        
        // Create and sign a message
        let message = b"test message";
        let signature = key_pair.sign(message).unwrap();
        
        // Verify signature
        let result = manager.verify_signature(
            &doc.id,
            "#key-1",
            message,
            &signature
        ).await.unwrap();
        
        assert!(result);
        
        // Test with invalid signature
        let invalid_sig = icn_crypto::Signature::new(vec![0; 64]);
        let result = manager.verify_signature(
            &doc.id,
            "#key-1",
            message,
            &invalid_sig
        ).await.unwrap();
        
        assert!(!result);
    }
    
    #[tokio::test]
    async fn test_authentication_flow() {
        let (manager, _temp) = create_test_manager().await;
        
        // Create a DID with key pair
        let (doc, key_pair) = manager.create_did(CreateDidOptions::default()).await.unwrap();
        
        // Create challenge
        let challenge = manager.create_authentication_challenge(
            &doc.id,
            None,
            None
        ).await.unwrap();
        
        // Sign challenge
        let signature = key_pair.sign(&challenge.get_message()).unwrap();
        
        // Create response
        let response = AuthenticationResponse {
            challenge,
            signature,
        };
        
        // Verify authentication
        let result = manager.verify_authentication(&response).await.unwrap();
        assert!(result);
    }

    #[tokio::test]
    async fn test_authentication() {
        let (manager, _temp) = create_test_manager().await;
        
        // Create a test DID with key pair
        let (did_doc, key_pair) = manager.create_did(CreateDidOptions::default()).await.unwrap();
        
        // Create authentication challenge
        let challenge = manager.create_authentication_challenge(
            &did_doc.id,
            None,
            None
        ).await.unwrap();
        
        // Sign the challenge
        let message = challenge.get_message();
        let signature = key_pair.sign(&message).unwrap();
        
        let response = AuthenticationResponse {
            challenge,
            signature,
        };
        
        // Verify the authentication
        let result = manager.verify_authentication(&response).await.unwrap();
        assert!(result);
    }

    #[tokio::test]
    async fn test_signature_verification() {
        let (manager, _temp) = create_test_manager().await;
        
        // Create a test DID
        let (did_doc, key_pair) = manager.create_did(CreateDidOptions::default()).await.unwrap();
        
        // Sign a test message
        let message = b"test message";
        let signature = key_pair.sign(message).unwrap();
        
        // Verify using the DID's verification method
        let result = manager.verify_signature(
            &did_doc.id,
            "#key-1",
            message,
            &signature
        ).await.unwrap();
        
        assert!(result);
    }
}