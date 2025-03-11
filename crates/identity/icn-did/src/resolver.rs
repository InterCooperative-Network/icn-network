//! DID resolver implementation for the ICN method
use async_trait::async_trait;
use icn_common::{Error, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use icn_storage_system::{Storage, StorageOptions};
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
        // Validate the document
        self.validate_document(&document)?;
        
        // Check if document already exists
        let existing = self.storage.get::<StoredDidDocument>(did).await?;
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
                deactivated: Some(false),
                version_id: Some("1".to_string()),
                next_version_id: None,
            }
        };
        
        // Store document with metadata
        let stored = StoredDidDocument {
            document,
            metadata,
        };
        
        self.storage.put(did, &stored).await
    }
    
    /// Update a DID document
    pub async fn update(&self, did: &str, document: DidDocument) -> Result<()> {
        // Validate the document
        self.validate_document(&document)?;

        // Ensure document exists
        if !self.storage.exists(did).await? {
            return Err(Error::not_found(format!("DID {} not found", did)));
        }
        
        // Store updated document
        self.store(did, document).await
    }
    
    /// Deactivate a DID
    pub async fn deactivate(&self, did: &str) -> Result<()> {
        let mut stored = self.storage.get::<StoredDidDocument>(did).await?
            .ok_or_else(|| Error::not_found(format!("DID {} not found", did)))?;
            
        // Update metadata
        stored.metadata.deactivated = Some(true);
        stored.metadata.updated = Some(chrono::Utc::now().to_rfc3339());
        
        // Store updated document
        self.storage.put(did, &stored).await
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
}

#[async_trait]
impl DidResolver for IcnDidResolver {
    async fn resolve(&self, did: &str) -> Result<ResolutionResult> {
        // Validate DID format
        if !did.starts_with(&format!("did:{}:", DID_METHOD)) {
            return Ok(ResolutionResult {
                did_document: None,
                resolution_metadata: ResolutionMetadata {
                    error: Some("invalidDid".to_string()),
                    ..Default::default()
                },
                document_metadata: DocumentMetadata::default(),
            });
        }
        
        // Look up the document
        match self.storage.get::<StoredDidDocument>(did).await? {
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
            id: "key-1".to_string(),
            type_: "Ed25519VerificationKey2020".to_string(),
            controller: did.to_string(),
            public_key: crate::PublicKeyMaterial::Ed25519VerificationKey2020(
                "BASE58_PUBLIC_KEY".to_string()
            ),
        });
        
        resolver.update(did, document.clone()).await.unwrap();
        
        // Resolve updated document
        let updated = resolver.resolve(did).await.unwrap();
        assert!(updated.did_document.is_some());
        assert_eq!(updated.did_document.unwrap().verification_method.len(), 1);
        
        // Deactivate the DID
        resolver.deactivate(did).await.unwrap();
        
        // Check deactivation
        let deactivated = resolver.resolve(did).await.unwrap();
        assert!(deactivated.document_metadata.deactivated.unwrap());
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
        resolver.store("test123", doc.clone()).await.unwrap();

        // Resolve the document
        let result = resolver.resolve("test123").await.unwrap();
        assert!(result.did_document.is_some());
        assert_eq!(result.did_document.unwrap().id, doc.id);

        // Update the document
        let mut updated_doc = doc.clone();
        updated_doc.add_service(crate::Service {
            id: "service-1".to_string(),
            type_: "TestService".to_string(),
            service_endpoint: "https://example.com".to_string(),
        });

        resolver.update("test123", updated_doc).await.unwrap();

        // Verify update
        let result = resolver.resolve("test123").await.unwrap();
        assert_eq!(result.did_document.unwrap().service.len(), 1);

        // Deactivate
        resolver.deactivate("test123").await.unwrap();

        // Verify deactivation
        let result = resolver.resolve("test123").await.unwrap();
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
}