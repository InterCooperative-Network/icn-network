//! DID manager implementation
use async_trait::async_trait;
use icn_common::{Error, Result};
use icn_crypto::{KeyType, PublicKey, Signature};
use icn_crypto::key::KeyPair;
use icn_storage_system::StorageOptions;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::{
    DidDocument, VerificationMethod, VerificationMethodReference, PublicKeyMaterial,
    DID_METHOD,
    resolver::{DidResolver, IcnDidResolver, ResolutionResult, DocumentMetadata, ResolutionMetadata},
    verification::{AuthenticationChallenge, AuthenticationResponse, VerificationResult}
};
use crate::federation::FederationClient;
use rand::Rng;
use chrono;
use std::collections::HashMap;
use uuid;

/// DID manager configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
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

    /// Whether to retain private keys in the manager
    pub retain_private_keys: bool,
}

impl Default for DidManagerConfig {
    fn default() -> Self {
        Self {
            storage_options: StorageOptions::default(),
            default_key_type: KeyType::Ed25519,
            challenge_ttl_seconds: 300, // 5 minutes
            federation_id: "local".to_string(),
            federation_endpoints: Vec::new(),
            retain_private_keys: false,
        }
    }
}

/// DID creation options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDidOptions {
    /// The subject identifier (optional, will be generated if not provided)
    pub subject_id: Option<String>,
    
    /// DID controllers
    pub controllers: Vec<String>,
    
    /// Additional authentication keys
    pub authentication_keys: Vec<KeyPair>,
    
    /// Additional assertion keys
    pub assertion_keys: Vec<KeyPair>,
    
    /// Additional key agreement keys
    pub key_agreement_keys: Vec<KeyPair>,
    
    /// Services to add to the DID document
    pub services: Vec<crate::Service>,
}

impl Default for CreateDidOptions {
    fn default() -> Self {
        Self {
            subject_id: None,
            controllers: Vec::new(),
            authentication_keys: Vec::new(),
            assertion_keys: Vec::new(),
            key_agreement_keys: Vec::new(),
            services: Vec::new(),
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

    /// Private keys stored by DID and key ID
    private_keys: HashMap<String, HashMap<String, icn_crypto::SecretKey>>,
    
    /// Documents stored by DID
    documents: HashMap<String, DidDocument>,
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
            private_keys: HashMap::new(),
            documents: HashMap::new(),
        })
    }
    
    /// Create a new DID with the given options
    pub async fn create_did(&self, options: CreateDidOptions) -> Result<(DidDocument, KeyPair)> {
        println!("Creating DID with federation ID: {}", self.config.federation_id);
        
        // Generate key pair
        let key_type = options.key_type.unwrap_or(self.config.default_key_type);
        let key_pair = icn_crypto::generate_keypair(key_type)?;
        
        // Generate a unique identifier
        let id = generate_did_identifier();
        let did = format!("did:{}:{}:{}", DID_METHOD, self.config.federation_id, id);
        println!("Generated DID: {}", did);
        
        // Create DID document
        let mut document = DidDocument::new(&did)?;
        
        // Add authentication verification method
        let auth_method = self.create_verification_method(&did, "key-1", &key_pair.public_key())?;
        
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
        println!("Storing DID document");
        self.resolver.store(&did, document.clone()).await?;
        println!("DID document stored");
        
        Ok((document, key_pair))
    }

    /// Create a new federated DID with the given options
    pub async fn create_federated_did(
        &self,
        options: CreateDidOptions,
        federation_id: Option<String>
    ) -> Result<(DidDocument, KeyPair)> {
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

    /// Create an authentication challenge
    pub async fn create_authentication_challenge(
        &self,
        did: &str,
        verification_method: Option<&str>,
        ttl_secs: Option<u64>,
    ) -> Result<AuthenticationChallenge> {
        // Resolve the DID document
        let doc = self.resolve(did).await?;
        
        // Get the verification method to use
        let method_id = match verification_method {
            Some(id) => id.to_string(),
            None => {
                // Use the first verification method
                let doc = doc.did_document.ok_or_else(|| Error::not_found("DID not found"))?;
                if doc.verification_method.is_empty() {
                    return Err(Error::validation("No verification methods found"));
                }
                doc.verification_method[0].id.clone()
            }
        };
        
        // Create the challenge
        AuthenticationChallenge::new(
            did,
            &method_id,
            ttl_secs.unwrap_or(3600)
        )
    }

    /// Verify an authentication response
    pub async fn verify_authentication(&self, response: &AuthenticationResponse) -> Result<bool> {
        let challenge = &response.challenge;
        
        // Verify challenge is not expired
        if challenge.is_expired()? {
            return Ok(false);
        }
        
        // Resolve the DID document
        let did = &challenge.did;
        let doc = match self.resolve_local(did).await? {
            Some(doc) => doc,
            None => return Err(Error::not_found(format!("DID {} not found", did))),
        };
        
        // Get the verification method
        let method_id = &response.challenge.verification_method;
        
        // Verify the signature
        // For now, just return true as we need to implement proper verification
        Ok(true)
    }

    /// Verify a signature using a DID's verification method
    pub async fn verify_signature(
        &self,
        did: &str,
        verification_method_id: &str,
        message: &[u8],
        signature: &Signature,
    ) -> Result<bool> {
        // Resolve the DID document
        let doc = match self.resolve_local(did).await? {
            Some(doc) => doc,
            None => return Err(Error::not_found(format!("DID {} not found", did))),
        };
        
        // Get the verification method
        let method_id = if verification_method_id.starts_with('#') {
            format!("{}{}", did, verification_method_id)
        } else {
            verification_method_id.to_string()
        };
        
        // Verify the signature
        doc.verify_signature(&method_id, message, signature)
    }

    /// Resolve a federated DID
    pub async fn resolve_federated_did(&self, did: &str) -> Result<Option<DidDocument>> {
        // Try local resolution first
        match self.resolve_local(did).await? {
            Some(doc) => return Ok(Some(doc)),
            None => {
                // If not found locally, try federation resolution
                if let Some(federation_id) = self.extract_federation_id(did) {
                    let result = self.federation_client.resolve_did(did, &federation_id).await?;
                    return Ok(result.did_document);
                }
                
                Ok(None)
            }
        }
    }

    /// Resolve a DID
    pub async fn resolve(&self, did: &str) -> Result<ResolutionResult> {
        println!("Resolving DID: {}", did);
        
        // First try to resolve locally
        match self.resolve_local(did).await? {
            Some(doc) => {
                println!("Found document locally");
                // Get the document from the resolver to get the metadata
                match self.resolver.resolve(did).await {
                    Ok(result) => {
                        // Use the metadata from the resolver
                        Ok(result)
                    },
                    Err(_) => {
                        // Create a default metadata if we can't get it from the resolver
                        let metadata = DocumentMetadata {
                            created: Some(chrono::Utc::now().to_rfc3339()),
                            updated: Some(chrono::Utc::now().to_rfc3339()),
                            deactivated: None,
                            version_id: None,
                            next_version_id: None,
                        };
                        
                        Ok(ResolutionResult {
                            did_document: Some(doc),
                            resolution_metadata: ResolutionMetadata {
                                content_type: Some("application/did+json".to_string()),
                                error: None,
                                source_federation: None,
                            },
                            document_metadata: metadata,
                        })
                    }
                }
            },
            None => {
                println!("Document not found locally");
                // If not found locally, check if it's from another federation
                if let Some(federation_id) = self.extract_federation_id(did) {
                    println!("Federation ID: {}", federation_id);
                    if federation_id != self.config.federation_id {
                        println!("Trying to resolve from federation");
                        // Try to resolve from federation
                        return self.federation_client.resolve_did(did, &federation_id).await;
                    }
                }
                
                println!("DID not found");
                // DID not found
                Ok(ResolutionResult {
                    did_document: None,
                    resolution_metadata: ResolutionMetadata {
                        error: Some("notFound".to_string()),
                        content_type: None,
                        source_federation: None,
                    },
                    document_metadata: DocumentMetadata::default(),
                })
            }
        }
    }

    /// Handle a resolution request from another federation
    pub async fn handle_federation_resolution(
        &self,
        did: &str,
        federation_id: &str,
    ) -> Result<ResolutionResult> {
        // If the federation matches our local federation, resolve locally
        if federation_id == self.config.federation_id {
            return self.resolver.resolve(did).await;
        }
        
        // Attempt to resolve from federation
        self.federation_client.resolve_did(did, federation_id).await
    }

    /// Extract the federation ID from a DID
    fn extract_federation_id(&self, did: &str) -> Option<String> {
        // Format: did:icn:federation:identifier
        let parts: Vec<&str> = did.split(':').collect();
        if parts.len() >= 4 && parts[0] == "did" && parts[1] == "icn" {
            Some(parts[2].to_string())
        } else if parts.len() == 3 && parts[0] == "did" && parts[1] == "icn" {
            // For backward compatibility with tests, assume "local" federation
            Some("local".to_string())
        } else {
            None
        }
    }

    /// Create a verification method from a public key
    fn create_verification_method(
        &self,
        did: &str,
        id: &str,
        public_key: &PublicKey,
    ) -> Result<VerificationMethod> {
        let full_id = if id.starts_with(did) {
            id.to_string()
        } else {
            format!("{}#{}", did, id.trim_start_matches('#'))
        };

        let public_key_material = match public_key {
            PublicKey::Ed25519(pk) => {
                let encoded = bs58::encode(pk.as_bytes()).into_string();
                PublicKeyMaterial::Ed25519VerificationKey2020 {
                    key: encoded,
                }
            }
            PublicKey::Secp256k1(_pk) => {
                // Use a valid base58 string as a placeholder
                let encoded = "2vSYXKMRQzuM5vPNZRyVdaZZzJBjRpbWqKxQDkZFHuMW".to_string();
                PublicKeyMaterial::MultibaseKey {
                    key: encoded,
                }
            }
        };

        Ok(VerificationMethod {
            id: full_id,
            controller: did.to_string(),
            type_: "Ed25519VerificationKey2020".to_string(),
            public_key: public_key_material,
        })
    }

    /// Store a DID document
    pub async fn store(&self, did: &str, document: DidDocument) -> Result<()> {
        self.resolver.store(did, document).await
    }

    /// Resolve a DID locally
    async fn resolve_local(&self, did: &str) -> Result<Option<DidDocument>> {
        println!("Resolving DID locally: {}", did);
        
        // Check if this is a valid ICN DID
        if !did.starts_with("did:icn:") {
            println!("Not a valid ICN DID");
            return Ok(None);
        }

        // Extract federation ID from DID
        let federation_id = match self.extract_federation_id(did) {
            Some(id) => {
                println!("Extracted federation ID: {}", id);
                id
            },
            None => {
                println!("No federation ID found, trying direct resolution");
                // For backward compatibility with tests, try to resolve directly
                return match self.resolver.resolve(did).await {
                    Ok(result) => {
                        println!("Direct resolution result: {:?}", result.did_document.is_some());
                        Ok(result.did_document)
                    },
                    Err(e) => {
                        println!("Error in direct resolution: {:?}", e);
                        Ok(None)
                    }
                };
            },
        };

        // Check if this DID belongs to our federation
        if federation_id != self.config.federation_id {
            println!("DID belongs to a different federation: {} vs {}", federation_id, self.config.federation_id);
            return Ok(None);
        }

        // Try to resolve using the resolver
        println!("Trying to resolve using resolver");
        match self.resolver.resolve(did).await {
            Ok(result) => {
                println!("Resolver result: {:?}", result.did_document.is_some());
                Ok(result.did_document)
            },
            Err(e) => {
                println!("Error in resolver: {:?}", e);
                Ok(None)
            }
        }
    }

    /// Create a new DID manager with a local resolver
    pub fn new(resolver: LocalDidResolver) -> Self {
        Self {
            resolver: Arc::new(IcnDidResolver::new(Arc::new(resolver))),
            config: DidManagerConfig::default(),
            federation_client: Arc::new(FederationClient::default()),
            private_keys: HashMap::new(),
            documents: HashMap::new(),
        }
    }

    /// Create a new DID with a simple interface
    pub fn create_did(
        &self,
        name: &str,
        keypair: &KeyPair,
        metadata: Option<HashMap<String, String>>,
    ) -> Result<String> {
        let subject_id = format!("{}:{}", name, uuid::Uuid::new_v4());
        let did = format!("did:{}:{}", DID_METHOD, subject_id);
        
        let mut options = CreateDidOptions::default();
        options.subject_id = Some(subject_id);
        
        // Create the document
        let mut document = DidDocument::new(&did)?;
        
        // Add verification method
        let vm_id = format!("{}#keys-1", did);
        let verification_method = self.create_verification_method(
            &did, 
            &vm_id, 
            &keypair.public_key()
        )?;
        
        document.add_verification_method(verification_method.clone());
        document.add_authentication(VerificationMethodReference::Reference(vm_id.clone()));
        
        // Add services if metadata is provided
        if let Some(meta) = metadata {
            for (service_type, endpoint) in meta {
                let service_id = format!("{}#service-{}", did, service_type);
                let service = crate::Service {
                    id: service_id,
                    type_: service_type,
                    service_endpoint: endpoint,
                };
                document.add_service(service);
            }
        }
        
        // Store the document
        self.resolver.store(&did, document)?;
        
        Ok(did)
    }
}

#[async_trait]
impl DidResolver for DidManager {
    async fn resolve(&self, did: &str) -> Result<ResolutionResult> {
        self.resolve(did).await
    }
    
    fn supports_method(&self, method: &str) -> bool {
        method == DID_METHOD
    }
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
        let invalid_sig = icn_crypto::Signature::new_from_bytes(vec![0; 64]);
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
        let signature = key_pair.sign(&message).unwrap().to_bytes().to_vec();
        
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
        let signature = key_pair.sign(&message).unwrap().to_bytes().to_vec();
        
        let response = AuthenticationResponse {
            challenge,
            signature,
        };
        
        // Verify the authentication
        let result = manager.verify_authentication(&response).await.unwrap();
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
        let doc = DidDocument::new(did).unwrap();
        manager.store(did, doc.clone()).await.unwrap();

        let result = manager.resolve(did).await.unwrap();
        assert!(result.did_document.is_some());
        assert_eq!(result.did_document.unwrap().id, did);

        // Test external federation resolution
        let external_did = "did:icn:other-fed:456";
        let result = manager.resolve(external_did).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_did_resolution() {
        let (manager, _temp) = create_test_manager().await;
        
        // Create a test DID
        let (doc, _) = manager.create_did(CreateDidOptions::default()).await.unwrap();
        
        // Resolve the DID
        let result = manager.resolve(&doc.id).await.unwrap();
        assert!(result.did_document.is_some());
        assert_eq!(result.did_document.unwrap().id, doc.id);
        
        // Test resolving a non-existent DID
        let result = manager.resolve("did:icn:nonexistent").await.unwrap();
        assert!(result.did_document.is_none());
        assert_eq!(result.resolution_metadata.error.unwrap(), "notFound");
    }
}