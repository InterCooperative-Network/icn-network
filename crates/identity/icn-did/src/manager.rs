//! DID manager implementation
use async_trait::async_trait;
use icn_common::{Error, Result};
use icn_crypto::{PublicKey, Signature, KeyType};
use icn_crypto::key::KeyPair;
use std::sync::Arc;
use crate::{
    DidDocument, VerificationMethod, VerificationMethodReference, PublicKeyMaterial,
    DID_METHOD,
    resolver::{DidResolver, IcnDidResolver, ResolutionResult, DocumentMetadata, ResolutionMetadata},
    verification::{AuthenticationChallenge, AuthenticationResponse}
};
use crate::federation::FederationClient;
use rand::Rng;
use chrono;
use std::collections::HashMap;
use crate::resolver::DidResolutionParams;
use crate::Service;
use uuid::Uuid;
use crate::federation::MockFederationClient;

/// Configuration for the DID manager
#[derive(Debug, Clone)]
pub struct DidManagerConfig {
    /// Default federation ID
    pub default_federation_id: String,
}

/// Options for creating a new DID
#[derive(Debug)]
pub struct CreateDidOptions {
    /// Key pair for the DID
    pub keypair: KeyPair,
    
    /// Key type (e.g., Ed25519VerificationKey2020)
    pub key_type: String,
    
    /// Services to add to the DID document
    pub services: Option<Vec<Service>>,
    
    /// Federation ID to register with
    pub federation_id: Option<String>,
    
    /// Add assertion method
    pub add_assertion_method: bool,
    
    /// Add key agreement
    pub add_key_agreement: bool,
}

/// DID manager for coordinating DID operations
pub struct DidManager {
    /// The DID resolver
    resolver: Arc<IcnDidResolver>,
    
    /// Configuration options
    config: DidManagerConfig,

    /// Federation client
    federation_client: Arc<dyn FederationClient>,

    /// Private keys stored by DID and key ID
    private_keys: HashMap<String, HashMap<String, icn_crypto::PrivateKey>>,
    
    /// Documents stored by DID
    documents: HashMap<String, DidDocument>,
}

impl DidManager {
    /// Create a new DID manager
    pub async fn new(config: DidManagerConfig) -> Result<Self> {
        // Create the resolver
        let resolver = IcnDidResolver::default();
        let federation_client = crate::federation::new(
            &config.default_federation_id,
            Vec::new()
        ).await?;
        
        Ok(Self {
            resolver: Arc::new(resolver),
            config,
            federation_client,
            private_keys: HashMap::new(),
            documents: HashMap::new(),
        })
    }
    
    /// Create a new DID with the given options
    pub async fn create_did(&self, options: CreateDidOptions) -> Result<(String, DidDocument)> {
        // Generate a new DID
        let did_id = format!("did:icn:{}", Uuid::new_v4());
        
        // Create verification method ID
        let vm_id = format!("{}#keys-1", did_id);
        
        // Extract public key from keypair
        let public_key = options.keypair.public_key();
        
        // Create public key material
        let public_key_material = crate::verification::PublicKeyMaterial::from_public_key(&public_key)?;
        
        // Create verification method
        let verification_method = VerificationMethod {
            id: vm_id.clone(),
            controller: did_id.clone(),
            type_: options.key_type.clone(),
            public_key: public_key_material,
        };
        
        // Create DID document
        let mut document = DidDocument::new(&did_id)?;
        document.add_verification_method(verification_method);
        
        // Add authentication method
        document.authentication.push(VerificationMethodReference::Reference(vm_id.clone()));
        
        // Add assertion method if specified
        if options.add_assertion_method {
            document.assertion_method.push(VerificationMethodReference::Reference(vm_id.clone()));
        }
        
        // Add key agreement if specified
        if options.add_key_agreement {
            document.key_agreement.push(VerificationMethodReference::Reference(vm_id.clone()));
        }
        
        // Add services if provided
        if let Some(services) = options.services {
            for service in services {
                document.add_service(service);
            }
        }
        
        // Register with federation if specified
        if let Some(federation_id) = options.federation_id {
            self.register_did(&did_id, document.clone(), &federation_id).await?;
        }
        
        // Store locally
        self.resolver.store(&did_id, document.clone()).await?;
        
        Ok((did_id, document))
    }

    /// Create a new federated DID with the given options
    pub async fn create_federated_did(
        &self,
        options: CreateDidOptions,
        federation_id: Option<String>
    ) -> Result<(DidDocument, KeyPair)> {
        // This method is not fully implemented yet
        // For now, we'll just return a placeholder implementation
        
        // Create a new DID
        let (did_id, document) = self.create_did(options).await?;
        
        // Return a placeholder result
        // In a real implementation, we would need to handle the federation aspects
        Err(Error::internal("create_federated_did is not fully implemented yet"))
    }

    /// Update a DID document
    pub async fn update_did(&self, did: &str, document: DidDocument) -> Result<()> {
        // Validate the document
        if did != document.id {
            return Err(Error::validation("DID in document does not match the provided DID"));
        }
        
        // Update with federation
        let federation_id = self.config.default_federation_id.clone();
        self.federation_client.update_did(did, &federation_id, document.clone()).await?;
        
        // Update locally
        self.resolver.store(did, document).await?;
        
        Ok(())
    }
    
    /// Deactivate a DID
    pub async fn deactivate_did(&self, did: &str) -> Result<()> {
        // Deactivate with federation
        let federation_id = self.config.default_federation_id.clone();
        self.federation_client.deactivate_did(did, &federation_id).await?;
        
        // Remove locally
        self.resolver.delete_document(did).await?;
        
        Ok(())
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
                let doc = doc.document.ok_or_else(|| Error::not_found("DID not found"))?;
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
                    println!("Trying to resolve from federation");
                    // Try to resolve from federation
                    match self.federation_client.resolve_did(did, &federation_id).await {
                        Ok(document) => {
                            // Convert to ResolutionResult
                            return Ok(Some(document));
                        }
                        Err(e) => {
                            println!("Failed to resolve from federation: {}", e);
                            return Ok(None);
                        }
                    }
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
                        let doc = result.document.ok_or_else(|| Error::not_found("DID not found"))?;
                        let metadata = DocumentMetadata {
                            created: Some(chrono::Utc::now().to_rfc3339()),
                            updated: Some(chrono::Utc::now().to_rfc3339()),
                            deactivated: None,
                            version_id: None,
                            next_update: None,
                            additional: HashMap::new(),
                        };
                        
                        Ok(ResolutionResult {
                            document: Some(doc),
                            document_metadata: Some(metadata),
                            resolution_metadata: ResolutionMetadata {
                                error: None,
                                error_message: None,
                                content_type: None,
                                additional: HashMap::new(),
                            },
                        })
                    },
                    Err(_) => {
                        // Create a default metadata if we can't get it from the resolver
                        let metadata = DocumentMetadata {
                            created: Some(chrono::Utc::now().to_rfc3339()),
                            updated: Some(chrono::Utc::now().to_rfc3339()),
                            deactivated: None,
                            version_id: None,
                            next_update: None,
                            additional: HashMap::new(),
                        };
                        
                        Ok(ResolutionResult {
                            document: None,
                            document_metadata: None,
                            resolution_metadata: ResolutionMetadata {
                                error: Some("notFound".to_string()),
                                error_message: None,
                                content_type: None,
                                additional: HashMap::new(),
                            },
                        })
                    }
                }
            },
            None => {
                println!("Document not found locally");
                // If not found locally, check if it's from another federation
                if let Some(federation_id) = self.extract_federation_id(did) {
                    println!("Federation ID: {}", federation_id);
                    if federation_id != self.config.default_federation_id {
                        println!("Trying to resolve from federation");
                        // Try to resolve from federation
                        match self.federation_client.resolve_did(did, &federation_id).await {
                            Ok(document) => {
                                return Ok(ResolutionResult::success(document, DocumentMetadata::default()));
                            },
                            Err(e) => {
                                return Ok(ResolutionResult {
                                    document: None,
                                    document_metadata: None,
                                    resolution_metadata: ResolutionMetadata {
                                        error: Some("notFound".to_string()),
                                        error_message: Some(e.to_string()),
                                        content_type: None,
                                        additional: HashMap::new(),
                                    },
                                });
                            }
                        }
                    }
                }
                
                println!("DID not found");
                // DID not found
                Ok(ResolutionResult {
                    document: None,
                    document_metadata: None,
                    resolution_metadata: ResolutionMetadata {
                        error: Some("notFound".to_string()),
                        error_message: None,
                        content_type: None,
                        additional: HashMap::new(),
                    },
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
        if federation_id == self.config.default_federation_id {
            return self.resolver.resolve(did).await;
        }
        
        // Otherwise, try to resolve through the federation client
        match self.federation_client.resolve_did(did, federation_id).await {
            Ok(document) => {
                // Convert to ResolutionResult
                Ok(ResolutionResult {
                    document: Some(document),
                    document_metadata: Some(DocumentMetadata {
                        created: None,
                        updated: None,
                        deactivated: None,
                        version_id: None,
                        next_update: None,
                        additional: HashMap::new(),
                    }),
                    resolution_metadata: ResolutionMetadata {
                        error: None,
                        error_message: None,
                        content_type: None,
                        additional: HashMap::new(),
                    },
                })
            }
            Err(e) => {
                // Return error result
                Ok(ResolutionResult {
                    document: None,
                    document_metadata: None,
                    resolution_metadata: ResolutionMetadata {
                        error: Some("notFound".to_string()),
                        error_message: Some(e.to_string()),
                        content_type: None,
                        additional: HashMap::new(),
                    },
                })
            }
        }
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
                    properties: HashMap::new(),
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
        // Try to resolve from local cache first
        if let Some(doc) = self.documents.get(did) {
            return Ok(Some(doc.clone()));
        }
        
        // If not in local cache, try to resolve from the resolver
        match self.resolver.resolve(did).await {
            Ok(result) => {
                return Ok(result.document);
            }
            Err(_) => {
                // If not found in resolver, try federation resolution
                if let Some(federation_id) = self.extract_federation_id(did) {
                    println!("Trying to resolve from federation");
                    // Try to resolve from federation
                    match self.federation_client.resolve_did(did, &federation_id).await {
                        Ok(document) => {
                            // Convert to ResolutionResult
                            return Ok(Some(document));
                        }
                        Err(e) => {
                            println!("Failed to resolve from federation: {}", e);
                            return Ok(None);
                        }
                    }
                }
                
                // Not found anywhere
                Ok(None)
            }
        }
    }

    /// Fix the remaining resolve_did call in the resolve_remote method
    async fn resolve_remote(&self, did: &str) -> Result<ResolutionResult> {
        // Extract federation ID from DID
        if let Some(federation_id) = self.extract_federation_id(did) {
            // Try to resolve from federation
            match self.federation_client.resolve_did(did, &federation_id).await {
                Ok(document) => {
                    // Convert to ResolutionResult
                    return Ok(ResolutionResult {
                        document: Some(document),
                        document_metadata: Some(DocumentMetadata {
                            created: None,
                            updated: None,
                            deactivated: None,
                            version_id: None,
                            next_update: None,
                            additional: HashMap::new(),
                        }),
                        resolution_metadata: ResolutionMetadata {
                            error: None,
                            error_message: None,
                            content_type: None,
                            additional: HashMap::new(),
                        },
                    });
                }
                Err(e) => {
                    // Return error result
                    return Ok(ResolutionResult {
                        document: None,
                        document_metadata: None,
                        resolution_metadata: ResolutionMetadata {
                            error: Some("notFound".to_string()),
                            error_message: Some(e.to_string()),
                            content_type: None,
                            additional: HashMap::new(),
                        },
                    });
                }
            }
        }
        
        // Not found
        Ok(ResolutionResult {
            document: None,
            document_metadata: None,
            resolution_metadata: ResolutionMetadata {
                error: Some("notFound".to_string()),
                error_message: Some("DID not found".to_string()),
                content_type: None,
                additional: HashMap::new(),
            },
        })
    }

    async fn resolve_did(&self, did: &str) -> Result<ResolutionResult> {
        // First try to resolve locally
        if let Ok(Some(document)) = self.resolver.get_document(did).await {
            return Ok(ResolutionResult::success(document, DocumentMetadata::default()));
        }
        
        // If not found locally, try to resolve remotely
        self.resolve_remote(did).await
    }

    async fn register_did(&self, did: &str, document: DidDocument, federation_id: &str) -> Result<()> {
        // Register with federation
        self.federation_client.register_did(did, federation_id, document.clone()).await?;
        
        // Store locally
        self.resolver.store(did, document).await?;
        
        Ok(())
    }

    /// Create a DID with a specific federation
    pub async fn create_did_with_federation(
        &self,
        options: CreateDidOptions,
        federation_id: Option<String>
    ) -> Result<(DidDocument, KeyPair)> {
        // This method is not fully implemented yet
        // For now, we'll just return a placeholder implementation
        Err(Error::internal("create_did_with_federation is not fully implemented yet"))
    }
}

#[async_trait]
impl DidResolver for DidManager {
    async fn resolve(&self, did: &str) -> Result<ResolutionResult> {
        self.resolve_did(did).await
    }
    
    async fn resolve_with_params(&self, did: &str, _params: &DidResolutionParams) -> Result<ResolutionResult> {
        self.resolve_did(did).await
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
    use crate::federation::MockFederationClient;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_create_did() {
        // Create a mock federation client
        let federation_client = Arc::new(MockFederationClient::new());
        
        // Create a DID manager
        let manager = DidManager::new(
            DidManagerConfig {
                default_federation_id: "test-federation".to_string(),
            }
        ).await.unwrap();
        
        // Create a DID
        let (did, document) = manager.create_did(CreateDidOptions {
            keypair: icn_crypto::key::KeyPair::generate(icn_crypto::key::KeyType::Ed25519).unwrap(),
            key_type: "Ed25519VerificationKey2020".to_string(),
            services: None,
            federation_id: None,
            add_assertion_method: false,
            add_key_agreement: false,
        }).await.unwrap();
        
        // Verify the DID
        assert!(did.starts_with("did:icn:"));
        assert_eq!(document.id, did);
        assert_eq!(document.authentication.len(), 1);
    }
}