//! DID manager implementation
use async_trait::async_trait;
use icn_common::{Error, Result};
use icn_crypto::{KeyPair, KeyType, PublicKey, Signature};
use icn_storage_system::StorageOptions;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::{
    DidDocument, VerificationMethod, VerificationMethodReference, PublicKeyMaterial,
    resolver::{DidResolver, IcnDidResolver, ResolutionResult},
    verification::{AuthenticationChallenge, AuthenticationResponse, VerificationResult},
    DID_METHOD,
};
use crate::federation::FederationClient;
use rand::{thread_rng, Rng};

/// DID manager configuration
#[derive(Debug, Clone)]
pub struct DidManagerConfig {
    /// Storage options for the DID resolver
    pub storage_options: StorageOptions,
    
    /// Default key type for new DIDs
    pub default_key_type: KeyType,
    
    /// Default challenge TTL in seconds
    pub challenge_ttl_seconds: u64,

    /// Federation ID
    pub federation_id: String,

    /// Federation endpoints
    pub federation_endpoints: Vec<String>,
}

impl Default for DidManagerConfig {
    fn default() -> Self {
        Self {
            storage_options: StorageOptions::default(),
            default_key_type: KeyType::Ed25519,
            challenge_ttl_seconds: 300, // 5 minutes
            federation_id: "local".to_string(),
            federation_endpoints: Vec::new(),
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

    /// Federation client
    federation_client: Arc<FederationClient>,
}

impl DidManager {
    /// Create a new DID manager
    pub async fn new(config: DidManagerConfig) -> Result<Self> {
        let resolver = IcnDidResolver::new(config.storage_options.clone()).await?;
        let federation_client = FederationClient::new(
            &config.federation_id,
            config.federation_endpoints.clone()
        ).await?;
        
        Ok(Self {
            resolver: Arc::new(resolver),
            config,
            federation_client: Arc::new(federation_client),
        })
    }
    
    /// Create a new DID with the given options
    pub async fn create_did(&self, options: CreateDidOptions) -> Result<(DidDocument, Box<dyn KeyPair>)> {
        // Generate key pair
        let key_type = options.key_type.unwrap_or(self.config.default_key_type);
        let key_pair = icn_crypto::generate_keypair(key_type)?;
        
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

    /// Create a new federated DID with the given options
    pub async fn create_federated_did(
        &self,
        options: CreateDidOptions,
        federation_id: Option<String>
    ) -> Result<(DidDocument, Box<dyn KeyPair>)> {
        let federation = federation_id.unwrap_or_else(|| self.config.federation_id.clone());
        
        // Create local DID first
        let (mut document, key_pair) = self.create_did(options).await?;
        
        // Add federation context
        document.add_context("https://w3id.org/did-federation/v1");
        document.add_service(crate::Service {
            id: format!("{}#federation", document.id),
            type_: "FederationEndpoint".to_string(),
            service_endpoint: format!("federation://{}", federation),
        });
        
        // Update the document
        self.update_did(&document.id, document.clone()).await?;
        
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

    /// Resolve a federated DID
    pub async fn resolve_federated_did(&self, did: &str) -> Result<Option<DidDocument>> {
        // Try local resolution first
        if let Ok(doc) = self.resolver.resolve(did).await {
            return Ok(Some(doc));
        }
        
        // If not found locally, try federation resolution
        if let Some(federation_id) = self.extract_federation_id(did) {
            return self.federation_client.resolve_did(did, &federation_id).await;
        }
        
        Ok(None)
    }

    /// Resolve a DID, including federated resolution
    pub async fn resolve(&self, did: &str) -> Result<ResolutionResult> {
        // Try local resolution first
        let result = self.resolver.resolve(did).await?;
        if result.did_document.is_some() {
            return Ok(result);
        }

        // If not found locally and it's a federated DID, try federation resolution
        if let Some(federation_id) = self.extract_federation_id(did) {
            if let Some(doc) = self.federation_client
                .resolve_did(did, &federation_id)
                .await? {
                return Ok(ResolutionResult {
                    did_document: Some(doc),
                    resolution_metadata: ResolutionMetadata {
                        content_type: Some("application/did+json".to_string()),
                        source_federation: Some(federation_id),
                        ..Default::default()
                    },
                    document_metadata: DocumentMetadata::default(),
                });
            }
        }

        // Not found in any federation
        Ok(ResolutionResult {
            did_document: None,
            resolution_metadata: ResolutionMetadata {
                error: Some("notFound".to_string()),
                ..Default::default()
            },
            document_metadata: DocumentMetadata::default(),
        })
    }

    /// Handle a resolution request from another federation
    pub async fn handle_federation_resolution(
        &self,
        did: &str,
        federation_id: &str,
    ) -> Result<ResolutionResult> {
        self.resolver.handle_federation_resolution(did, federation_id).await
    }

    fn extract_federation_id(&self, did: &str) -> Option<String> {
        // Format: did:icn:<federation>:<identifier>
        let parts: Vec<&str> = did.split(':').collect();
        if parts.len() == 4 && parts[0] == "did" && parts[1] == "icn" {
            Some(parts[2].to_string())
        } else {
            None
        }
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
        let message = challenge.get_message();
        let signature = key_pair.sign(&message).unwrap();
        
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

    #[tokio::test]
    async fn test_federated_resolution() {
        let temp_dir = tempdir().unwrap();
        let config = DidManagerConfig {
            storage_options: StorageOptions {
                base_dir: temp_dir.path().to_path_buf(),
                sync_writes: true,
                compress: false,
            },
            federation_id: "test-fed".to_string(),
            federation_endpoints: vec![
                "http://federation1.example.com".to_string(),
                "http://federation2.example.com".to_string(),
            ],
            ..Default::default()
        };

        let manager = DidManager::new(config).await.unwrap();

        // Test local federation resolution
        let did = "did:icn:test-fed:123";
        let doc = DidDocument::new("test-fed:123").unwrap();
        manager.store(did, doc.clone()).await.unwrap();

        let result = manager.resolve(did).await.unwrap();
        assert!(result.did_document.is_some());
        assert_eq!(result.did_document.unwrap().id, did);

        // Test external federation resolution
        let external_did = "did:icn:other-fed:456";
        let result = manager.resolve(external_did).await.unwrap();
        assert!(result.did_document.is_none());
        assert_eq!(result.resolution_metadata.error.unwrap(), "notFound");

        // Test federation resolution request handling
        let result = manager
            .handle_federation_resolution(did, "test-fed")
            .await
            .unwrap();
        assert!(result.did_document.is_some());
        assert_eq!(
            result.resolution_metadata.source_federation.unwrap(),
            "test-fed"
        );
    }

    #[tokio::test]
    async fn test_federation_caching() {
        let temp_dir = tempdir().unwrap();
        let config = DidManagerConfig {
            storage_options: StorageOptions {
                base_dir: temp_dir.path().to_path_buf(),
                sync_writes: true,
                compress: false,
            },
            federation_id: "test-fed".to_string(),
            federation_endpoints: vec!["http://localhost:8080".to_string()],
            ..Default::default()
        };

        let manager = DidManager::new(config).await.unwrap();

        // Create a test DID document
        let did = "did:icn:test-fed:123";
        let doc = DidDocument::new("test-fed:123").unwrap();

        // Store in federation client cache
        manager
            .federation_client
            .cache_document(did, doc.clone(), 1)
            .await
            .unwrap();

        // First resolution should use cache
        let result = manager.resolve(did).await.unwrap();
        assert!(result.did_document.is_some());

        // Wait for cache to expire
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        // Second resolution should not find document
        let result = manager.resolve(did).await.unwrap();
        assert!(result.did_document.is_none());
    }
}

use crate::{
    DidDocument, DidResolutionMetadata, DidDocumentMetadata,
    resolver::{DidResolver, ResolutionResult},
    federation::{FederationClient, FederationDidResolver},
    verification::{DidVerifier, KeyVerifier},
};
use icn_common::{Error, Result};
use icn_crypto::{Signature, KeyPair};
use std::sync::Arc;
use tokio::sync::{RwLock, Mutex};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// DID Manager for local and cross-federation DID operations
pub struct DidManager {
    /// DID resolver
    resolver: Arc<dyn DidResolver>,
    
    /// DID verifier
    verifier: Arc<dyn DidVerifier>,
    
    /// Local federation ID
    federation_id: String,
    
    /// Federation client for cross-federation operations
    federation_client: Arc<FederationClient>,
    
    /// Document cache
    document_cache: Arc<RwLock<HashMap<String, CachedDocument>>>,
    
    /// Verification cache for performance optimization
    #[allow(clippy::type_complexity)]
    verification_cache: Arc<Mutex<HashMap<String, CachedVerification>>>,
    
    /// Document cache TTL
    cache_ttl: Duration,
}

/// Cached DID document
struct CachedDocument {
    /// Resolution result
    result: ResolutionResult,
    /// Expiration time
    expires_at: Instant,
}

/// Cached verification result
struct CachedVerification {
    /// Verification result
    is_valid: bool,
    /// Expiration time
    expires_at: Instant,
}

impl DidManager {
    /// Create a new DID manager
    pub fn new(
        resolver: Arc<dyn DidResolver>,
        verifier: Arc<dyn DidVerifier>,
        federation_id: String,
    ) -> Self {
        let federation_client = Arc::new(FederationClient::new(federation_id.clone()));
        
        Self {
            resolver,
            verifier,
            federation_id,
            federation_client,
            document_cache: Arc::new(RwLock::new(HashMap::new())),
            verification_cache: Arc::new(Mutex::new(HashMap::new())),
            cache_ttl: Duration::from_secs(3600),
        }
    }
    
    /// Create a DID manager with federation support
    pub fn with_federation_support(
        local_resolver: Arc<dyn DidResolver>,
        verifier: Arc<dyn DidVerifier>,
        federation_id: String,
    ) -> Self {
        let federation_client = Arc::new(FederationClient::new(federation_id.clone()));
        let fed_resolver = FederationDidResolver::new(federation_client.clone())
            .with_local_resolver(local_resolver);
            
        Self {
            resolver: Arc::new(fed_resolver),
            verifier,
            federation_id,
            federation_client,
            document_cache: Arc::new(RwLock::new(HashMap::new())),
            verification_cache: Arc::new(Mutex::new(HashMap::new())),
            cache_ttl: Duration::from_secs(3600),
        }
    }
    
    /// Get local federation ID
    pub fn federation_id(&self) -> &str {
        &self.federation_id
    }
    
    /// Get federation client
    pub fn federation_client(&self) -> Arc<FederationClient> {
        self.federation_client.clone()
    }
    
    /// Set document cache TTL
    pub fn set_cache_ttl(&mut self, ttl: Duration) {
        self.cache_ttl = ttl;
    }
    
    /// Resolve a DID
    pub async fn resolve(&self, did: &str) -> Result<ResolutionResult> {
        // Check cache first
        if let Some(cached) = self.get_cached_document(did).await {
            return Ok(cached);
        }
        
        // Resolve DID
        let result = self.resolver.resolve(did).await?;
        
        // Cache if successful
        if result.did_document.is_some() {
            self.cache_document(did, result.clone()).await;
        }
        
        Ok(result)
    }
    
    /// Get cached document
    async fn get_cached_document(&self, did: &str) -> Option<ResolutionResult> {
        let cache = self.document_cache.read().await;
        if let Some(entry) = cache.get(did) {
            if entry.expires_at > Instant::now() {
                return Some(entry.result.clone());
            }
        }
        None
    }
    
    /// Cache document
    async fn cache_document(&self, did: &str, result: ResolutionResult) {
        let mut cache = self.document_cache.write().await;
        cache.insert(did.to_string(), CachedDocument {
            result,
            expires_at: Instant::now() + self.cache_ttl,
        });
    }
    
    /// Verify a DID signature
    pub async fn verify_signature(
        &self,
        did: &str,
        challenge: &[u8],
        signature: &Signature,
    ) -> Result<bool> {
        // Check cache first
        let cache_key = format!("{}:{}:{}", did, hex::encode(challenge), hex::encode(signature.as_bytes()));
        if let Some(cached) = self.get_cached_verification(&cache_key).await {
            return Ok(cached);
        }
        
        // Check if it's a cross-federation DID
        let parts: Vec<&str> = did.split(':').collect();
        if parts.len() >= 4 && parts[0] == "did" && parts[1] == "icn" && parts[2] != self.federation_id {
            // Cross-federation verification
            let target_federation = parts[2];
            let is_valid = self.federation_client
                .verify_signature(did, target_federation, challenge, signature)
                .await?;
                
            // Cache result
            self.cache_verification(&cache_key, is_valid).await;
            return Ok(is_valid);
        }
        
        // Local verification
        let result = self.resolve(did).await?;
        
        if let Some(document) = result.did_document {
            let is_valid = self.verifier.verify(&document, challenge, signature).await?;
            
            // Cache result
            self.cache_verification(&cache_key, is_valid).await;
            
            Ok(is_valid)
        } else {
            Err(Error::new(format!("DID document not found for {}", did)))
        }
    }
    
    /// Get cached verification
    async fn get_cached_verification(&self, key: &str) -> Option<bool> {
        let cache = self.verification_cache.lock().await;
        if let Some(entry) = cache.get(key) {
            if entry.expires_at > Instant::now() {
                return Some(entry.is_valid);
            }
        }
        None
    }
    
    /// Cache verification
    async fn cache_verification(&self, key: &str, is_valid: bool) {
        let mut cache = self.verification_cache.lock().await;
        cache.insert(key.to_string(), CachedVerification {
            is_valid,
            expires_at: Instant::now() + self.cache_ttl,
        });
    }
    
    /// Clear expired cache entries
    pub async fn clear_expired_cache(&self) {
        // Clear document cache
        {
            let mut cache = self.document_cache.write().await;
            let now = Instant::now();
            cache.retain(|_, entry| entry.expires_at > now);
        }
        
        // Clear verification cache
        {
            let mut cache = self.verification_cache.lock().await;
            let now = Instant::now();
            cache.retain(|_, entry| entry.expires_at > now);
        }
    }
    
    /// Register federation endpoints
    pub async fn register_federation_endpoints(&self, federation_id: &str, endpoints: Vec<String>) -> Result<()> {
        self.federation_client.add_federation_endpoints(federation_id, endpoints).await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolver::{InMemoryDidResolver};
    use crate::verification::{KeyVerifier};
    use icn_crypto::ed25519::{Ed25519KeyPair, Ed25519Signature};
    use std::collections::HashMap;
    
    #[tokio::test]
    async fn test_did_manager() {
        // Create local resolver and verifier
        let resolver = Arc::new(InMemoryDidResolver::new());
        let verifier = Arc::new(KeyVerifier::new());
        
        // Create DID manager
        let manager = DidManager::with_federation_support(
            resolver,
            verifier,
            "test-federation".to_string(),
        );
        
        // Test federation ID
        assert_eq!(manager.federation_id(), "test-federation");
        
        // Test cache TTL
        let mut manager_with_ttl = DidManager::new(
            Arc::new(InMemoryDidResolver::new()),
            Arc::new(KeyVerifier::new()),
            "test-federation".to_string(),
        );
        manager_with_ttl.set_cache_ttl(Duration::from_secs(30));
        
        // Test federation endpoints registration
        manager.register_federation_endpoints(
            "partner-federation",
            vec!["https://partner.example.com/api".to_string()],
        ).await.unwrap();
    }
    
    #[tokio::test]
    async fn test_did_resolution_and_verification() {
        // Create key pair for testing
        let key_pair = Ed25519KeyPair::generate();
        let public_key = key_pair.public_key();
        
        // Create a DID document
        let did = format!("did:icn:test-federation:{}", hex::encode(public_key.as_bytes()));
        let document = DidDocument {
            id: did.clone(),
            controller: vec![did.clone()],
            authentication: vec![format!("{}#keys-1", did)],
            verification_method: vec![
                crate::DidVerificationMethod {
                    id: format!("{}#keys-1", did),
                    controller: did.clone(),
                    type_: "Ed25519VerificationKey2020".to_string(),
                    public_key_multibase: format!("z{}", hex::encode(public_key.as_bytes())),
                }
            ],
            assertion_method: Vec::new(),
            key_agreement: Vec::new(),
            capability_invocation: Vec::new(),
            capability_delegation: Vec::new(),
            service: Vec::new(),
        };
        
        // Create a resolver with the document
        let mut resolver = InMemoryDidResolver::new();
        resolver.add_document(did.clone(), document);
        
        // Create DID manager
        let manager = DidManager::new(
            Arc::new(resolver),
            Arc::new(KeyVerifier::new()),
            "test-federation".to_string(),
        );
        
        // Test resolution
        let result = manager.resolve(&did).await.unwrap();
        assert!(result.did_document.is_some());
        
        // Test signature verification
        let challenge = b"test challenge";
        let signature = key_pair.sign(challenge);
        
        let is_valid = manager.verify_signature(&did, challenge, &signature).await.unwrap();
        assert!(is_valid);
        
        // Test caching - should use cached document
        let _cached_result = manager.resolve(&did).await.unwrap();
        
        // Test cache clearing
        manager.clear_expired_cache().await;
    }
}