//! DID resolver implementation for the ICN method
use async_trait::async_trait;
use icn_common::{Error, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use icn_storage_system::{Storage, StorageOptions, StorageExt};
use crate::{DidDocument, DID_METHOD};

/// Resolution metadata according to the DID Resolution spec
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResolutionMetadata {
    /// Content type of the resolved document
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    
    /// Error during resolution
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    /// Source federation ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_federation: Option<String>,
}

/// DID document metadata according to the DID Core spec
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DocumentMetadata {
    /// When the DID document was created
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<String>,
    
    /// When the DID document was last updated
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated: Option<String>,
    
    /// Whether the DID has been deactivated
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deactivated: Option<bool>,
    
    /// Version ID of the DID document
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_id: Option<String>,
    
    /// Next version ID of the DID document
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_version_id: Option<String>,
}

/// Result of resolving a DID
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolutionResult {
    /// The resolved DID document
    #[serde(skip_serializing_if = "Option::is_none")]
    pub did_document: Option<DidDocument>,
    
    /// Metadata about the resolution process
    pub resolution_metadata: ResolutionMetadata,
    
    /// Metadata about the DID document
    pub document_metadata: DocumentMetadata,
}

/// Interface for DID resolution
#[async_trait]
pub trait DidResolver: Send + Sync {
    /// Resolve a DID to a DID document
    async fn resolve(&self, did: &str) -> Result<ResolutionResult>;
    
    /// Check if this resolver supports a given DID method
    fn supports_method(&self, method: &str) -> bool;
}

/// Stored DID document with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredDidDocument {
    /// The DID document
    document: DidDocument,
    /// Document metadata
    metadata: DocumentMetadata,
}

/// The ICN DID resolver with persistent storage
pub struct IcnDidResolver {
    /// Storage for DID documents
    storage: Arc<dyn Storage>,
}

impl IcnDidResolver {
    /// Create a new ICN DID resolver with the given storage options
    pub async fn new(storage_options: StorageOptions) -> Result<Self> {
        let storage = icn_storage_system::create_storage(storage_options).await?;
        Ok(Self { storage })
    }
    
    /// Store a DID document
    pub async fn store(&self, did: &str, document: DidDocument) -> Result<()> {
        println!("Storing DID document for: {}", did);
        
        // Validate the document
        self.validate_document(&document)?;
        
        // Ensure we have a consistent storage key format
        let storage_key = normalize_did(did);
        println!("Storage key: {}", storage_key);
        
        // Check if document already exists
        let existing = self.storage.get::<StoredDidDocument>(&storage_key).await?;
        println!("Document exists: {}", existing.is_some());
        
        let metadata = if let Some(existing) = existing {
            // Update metadata for existing document
            DocumentMetadata {
                created: existing.metadata.created,
                updated: Some(chrono::Utc::now().to_rfc3339()),
                deactivated: existing.metadata.deactivated,
                version_id: Some((existing.metadata.version_id.unwrap_or("0".to_string())
                    .parse::<u64>()
                    .unwrap_or(0) + 1)
                    .to_string()),
                next_version_id: None,
            }
        } else {
            // Create new metadata
            DocumentMetadata {
                created: Some(chrono::Utc::now().to_rfc3339()),
                updated: Some(chrono::Utc::now().to_rfc3339()),
                deactivated: None,
                version_id: Some("1".to_string()),
                next_version_id: None,
            }
        };
        
        // Store the document with metadata
        let stored = StoredDidDocument {
            document,
            metadata,
        };
        
        println!("Putting document in storage");
        self.storage.put(&storage_key, &stored).await?;
        println!("Document stored successfully");
        
        Ok(())
    }
    
    /// Update a DID document
    pub async fn update(&self, did: &str, document: DidDocument) -> Result<()> {
        // Validate the document
        self.validate_document(&document)?;

        // Ensure we have a consistent storage key format
        let storage_key = normalize_did(did);

        // Ensure document exists
        if !self.storage.exists(&storage_key).await? {
            return Err(Error::not_found(format!("DID {} not found", did)));
        }
        
        // Store updated document
        self.store(did, document).await
    }
    
    /// Deactivate a DID
    pub async fn deactivate(&self, did: &str) -> Result<()> {
        // Ensure we have a consistent storage key format
        let storage_key = normalize_did(did);

        let mut stored = self.storage.get::<StoredDidDocument>(&storage_key).await?
            .ok_or_else(|| Error::not_found(format!("DID {} not found", did)))?;
            
        // Update metadata
        stored.metadata.deactivated = Some(true);
        stored.metadata.updated = Some(chrono::Utc::now().to_rfc3339());
        
        // Store updated document
        self.storage.put(&storage_key, &stored).await
    }
    
    /// List all DIDs
    pub async fn list_dids(&self) -> Result<Vec<String>> {
        self.storage.list_keys("").await
    }

    fn validate_document(&self, document: &DidDocument) -> Result<()> {
        // Basic validation
        if document.id.is_empty() {
            return Err(Error::validation("DID document must have an id"));
        }

        // Validate verification methods
        for method in &document.verification_method {
            if method.id.is_empty() {
                return Err(Error::validation("Verification method must have an id"));
            }
            if method.type_.is_empty() {
                return Err(Error::validation("Verification method must have a type"));
            }
            if method.controller.is_empty() {
                return Err(Error::validation("Verification method must have a controller"));
            }

            // Validate public key material based on type
            match method.type_.as_str() {
                "Ed25519VerificationKey2020" => {
                    // Ensure we can decode the key
                    if let Err(e) = bs58::decode(method.public_key.to_string())
                        .into_vec()
                    {
                        return Err(Error::validation(
                            format!("Invalid Ed25519 public key: {}", e)
                        ));
                    }
                }
                // Add validation for other key types as needed
                _ => return Err(Error::validation("Unsupported verification method type")),
            }
        }

        Ok(())
    }

    /// Handle a resolution request from another federation
    pub async fn handle_federation_resolution(
        &self,
        did: &str,
        federation_id: &str,
    ) -> Result<ResolutionResult> {
        // Extract federation ID from DID
        let parts: Vec<&str> = did.split(':').collect();
        if parts.len() != 4 || parts[0] != "did" || parts[1] != "icn" {
            return Ok(ResolutionResult {
                did_document: None,
                resolution_metadata: ResolutionMetadata {
                    error: Some("Invalid DID format".to_string()),
                    content_type: None,
                    source_federation: Some(federation_id.to_string()),
                },
                document_metadata: DocumentMetadata {
                    created: None,
                    updated: None,
                    deactivated: None,
                    version_id: None,
                    next_version_id: None,
                },
            });
        }

        // Check if this DID belongs to our federation
        let did_federation = parts[2];
        if did_federation != federation_id {
            return Ok(ResolutionResult {
                did_document: None,
                resolution_metadata: ResolutionMetadata {
                    error: Some("DID not found in this federation".to_string()),
                    content_type: None,
                    source_federation: Some(federation_id.to_string()),
                },
                document_metadata: DocumentMetadata {
                    created: None,
                    updated: None,
                    deactivated: None,
                    version_id: None,
                    next_version_id: None,
                },
            });
        }

        // Resolve locally
        self.resolve(did).await
    }
}

#[async_trait]
impl DidResolver for IcnDidResolver {
    async fn resolve(&self, did: &str) -> Result<ResolutionResult> {
        println!("Resolver: Resolving DID: {}", did);
        
        // Validate the DID
        if !did.starts_with("did:icn:") {
            println!("Resolver: Invalid DID format");
            return Ok(ResolutionResult {
                did_document: None,
                resolution_metadata: ResolutionMetadata {
                    error: Some("invalidDid".to_string()),
                    ..Default::default()
                },
                document_metadata: DocumentMetadata::default(),
            });
        }
        
        // Ensure we have a consistent storage key format
        let storage_key = normalize_did(did);
        println!("Resolver: Storage key: {}", storage_key);
        
        // Look up the document
        let stored_doc = self.storage.get::<StoredDidDocument>(&storage_key).await?;
        println!("Resolver: Document found: {}", stored_doc.is_some());
        
        match stored_doc {
            Some(stored) => Ok(ResolutionResult {
                did_document: Some(stored.document),
                resolution_metadata: ResolutionMetadata {
                    content_type: Some("application/did+json".to_string()),
                    ..Default::default()
                },
                document_metadata: stored.metadata,
            }),
            None => Ok(ResolutionResult {
                did_document: None,
                resolution_metadata: ResolutionMetadata {
                    error: Some("notFound".to_string()),
                    ..Default::default()
                },
                document_metadata: DocumentMetadata::default(),
            }),
        }
    }
    
    fn supports_method(&self, method: &str) -> bool {
        method == DID_METHOD
    }
}

/// Create a new ICN DID resolver
pub async fn create_resolver(storage_options: StorageOptions) -> Result<Arc<IcnDidResolver>> {
    let resolver = IcnDidResolver::new(storage_options).await?;
    Ok(Arc::new(resolver))
}

/// Helper function to normalize a DID for storage
fn normalize_did(did: &str) -> String {
    println!("Normalizing DID: {}", did);
    
    // If it's already a fully qualified DID, return it as is
    if did.starts_with(&format!("did:{}:", DID_METHOD)) {
        println!("DID is already fully qualified");
        did.to_string()
    } else {
        // If it's just an identifier, assume it's for the default method
        // This is mainly for backward compatibility with tests
        let normalized = format!("did:{}:local:{}", DID_METHOD, did);
        println!("Normalized DID: {}", normalized);
        normalized
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[tokio::test]
    async fn test_resolver_basic_operations() {
        let temp_dir = tempdir().unwrap();
        let options = StorageOptions {
            base_dir: temp_dir.path().to_path_buf(),
            sync_writes: true,
            compress: false,
        };
        
        let resolver = IcnDidResolver::new(options).await.unwrap();
        let did = "did:icn:test123";
        
        // Create and store a document
        let mut document = DidDocument::new("test123").unwrap();
        resolver.store(did, document.clone()).await.unwrap();
        
        // Resolve the document
        let result = resolver.resolve(did).await.unwrap();
        assert!(result.did_document.is_some());
        assert_eq!(result.did_document.unwrap().id, did);
        
        // Update the document
        document.add_verification_method(crate::VerificationMethod {
            id: format!("{}#key-1", did),
            type_: "Ed25519VerificationKey2020".to_string(),
            controller: did.to_string(),
            public_key: crate::PublicKeyMaterial::Ed25519VerificationKey2020 {
                key: "11111111111111111111111111111111".to_string() // Valid base58 string
            },
        });
        
        resolver.update(did, document.clone()).await.unwrap();
        
        // Resolve updated document
        let updated = resolver.resolve(did).await.unwrap();
        assert!(updated.did_document.is_some());
        assert_eq!(updated.did_document.unwrap().verification_method.len(), 1);
        
        // Test non-existent DID
        let not_found = resolver.resolve("did:icn:nonexistent").await.unwrap();
        assert!(not_found.did_document.is_none());
        assert_eq!(not_found.resolution_metadata.error.unwrap(), "notFound");
    }
    
    #[tokio::test]
    async fn test_resolver_invalid_did() {
        let temp_dir = tempdir().unwrap();
        let options = StorageOptions {
            base_dir: temp_dir.path().to_path_buf(),
            sync_writes: true,
            compress: false,
        };
        
        let resolver = IcnDidResolver::new(options).await.unwrap();
        
        // Test invalid DID format
        let result = resolver.resolve("invalid:did").await.unwrap();
        assert!(result.did_document.is_none());
        assert_eq!(
            result.resolution_metadata.error.unwrap(),
            "invalidDid"
        );
        
        // Test non-existent DID
        let result = resolver.resolve("did:icn:nonexistent").await.unwrap();
        assert!(result.did_document.is_none());
        assert_eq!(
            result.resolution_metadata.error.unwrap(),
            "notFound"
        );
    }

    #[tokio::test]
    async fn test_resolver_lifecycle() {
        let temp_dir = tempdir().unwrap();
        let storage_options = StorageOptions {
            base_dir: temp_dir.path().to_path_buf(),
            sync_writes: true,
            compress: false,
        };

        let resolver = IcnDidResolver::new(storage_options).await.unwrap();

        // Create a test document
        let doc = DidDocument::new("test123").unwrap();
        let did = "did:icn:test123";
        resolver.store(did, doc.clone()).await.unwrap();

        // Resolve the document
        let result = resolver.resolve(did).await.unwrap();
        assert!(result.did_document.is_some());
        assert_eq!(result.did_document.unwrap().id, doc.id);

        // Update the document
        let mut updated_doc = doc.clone();
        updated_doc.add_service(crate::Service {
            id: format!("{}#service-1", did),
            type_: "TestService".to_string(),
            service_endpoint: "https://example.com".to_string(),
        });

        resolver.update(did, updated_doc.clone()).await.unwrap();

        // Resolve updated document
        let result = resolver.resolve(did).await.unwrap();
        assert!(result.did_document.is_some());
        let resolved_doc = result.did_document.unwrap();
        assert_eq!(resolved_doc.service.len(), 1);
        assert_eq!(resolved_doc.service[0].type_, "TestService");

        // Deactivate the document
        resolver.deactivate(did).await.unwrap();

        // Resolve deactivated document
        let result = resolver.resolve(did).await.unwrap();
        assert!(result.did_document.is_some());
        assert!(result.document_metadata.deactivated.unwrap());
    }

    #[tokio::test]
    async fn test_document_validation() {
        let temp_dir = tempdir().unwrap();
        let storage_options = StorageOptions {
            base_dir: temp_dir.path().to_path_buf(),
            sync_writes: true,
            compress: false,
        };

        let resolver = IcnDidResolver::new(storage_options).await.unwrap();

        // Test storing invalid document
        let mut doc = DidDocument::new("test123").unwrap();
        doc.id = "".to_string(); // Invalid - empty ID
        assert!(resolver.store("test123", doc).await.is_err());
    }

    #[tokio::test]
    async fn test_list_dids() {
        let temp_dir = tempdir().unwrap();
        let storage_options = StorageOptions {
            base_dir: temp_dir.path().to_path_buf(),
            sync_writes: true,
            compress: false,
        };

        let resolver = IcnDidResolver::new(storage_options).await.unwrap();

        // Create multiple DIDs
        for i in 0..3 {
            let doc = DidDocument::new(&format!("test{}", i)).unwrap();
            resolver.store(&format!("test{}", i), doc).await.unwrap();
        }

        let dids = resolver.list_dids().await.unwrap();
        assert_eq!(dids.len(), 3);
    }

    #[tokio::test]
    async fn test_federation_resolution() {
        let temp_dir = tempdir().unwrap();
        let storage_options = StorageOptions {
            base_dir: temp_dir.path().to_path_buf(),
            sync_writes: true,
            compress: false,
        };

        let resolver = IcnDidResolver::new(storage_options).await.unwrap();
        
        // Test with local federation DID
        let did = "did:icn:test-fed:123";
        let resolution = resolver
            .handle_federation_resolution(did, "test-fed")
            .await
            .unwrap();
            
        assert!(resolution.did_document.is_none());
        assert!(resolution.resolution_metadata.error.is_some());
        
        // Test with different federation DID
        let did = "did:icn:other-fed:123";
        let resolution = resolver
            .handle_federation_resolution(did, "test-fed")
            .await
            .unwrap();
            
        assert!(resolution.did_document.is_none());
        assert_eq!(
            resolution.resolution_metadata.error.unwrap(),
            "DID not found in this federation"
        );
        
        // Test with invalid DID
        let did = "invalid:did";
        let resolution = resolver
            .handle_federation_resolution(did, "test-fed")
            .await
            .unwrap();
            
        assert!(resolution.did_document.is_none());
        assert_eq!(
            resolution.resolution_metadata.error.unwrap(),
            "Invalid DID format"
        );
    }
}