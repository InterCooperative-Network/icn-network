//! DID resolver implementation for the ICN method
use async_trait::async_trait;
use icn_common::{Error, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use icn_storage_system::{Storage, StorageOptions, StorageExt};
use crate::{DidDocument, DID_METHOD};
use crate::federation::FederationClient;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Result of a DID resolution operation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResolutionResult {
    /// The resolved DID document, if successful
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document: Option<DidDocument>,
    
    /// Metadata about the DID document
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_metadata: Option<DocumentMetadata>,
    
    /// Metadata about the resolution process
    pub resolution_metadata: ResolutionMetadata,
}

/// Metadata about a DID document
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DocumentMetadata {
    /// When the DID document was created
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<String>,
    
    /// When the DID document was last updated
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated: Option<String>,
    
    /// Whether the DID document is deactivated
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deactivated: Option<bool>,
    
    /// The version ID of the DID document
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_id: Option<String>,
    
    /// The next update commitment for the DID document
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_update: Option<String>,
    
    /// Additional metadata
    #[serde(flatten)]
    pub additional: HashMap<String, serde_json::Value>,
}

impl Default for DocumentMetadata {
    fn default() -> Self {
        Self {
            created: None,
            updated: None,
            deactivated: None,
            version_id: None,
            next_update: None,
            additional: HashMap::new(),
        }
    }
}

/// Metadata about the resolution process
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResolutionMetadata {
    /// The error code if resolution failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    
    /// A human-readable message describing the error
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    
    /// The content type of the resolved DID document
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    
    /// Additional metadata
    #[serde(flatten)]
    pub additional: HashMap<String, serde_json::Value>,
}

impl Default for ResolutionMetadata {
    fn default() -> Self {
        Self {
            error: None,
            error_message: None,
            content_type: Some("application/did+json".to_string()),
            additional: HashMap::new(),
        }
    }
}

impl ResolutionResult {
    /// Create a successful resolution result
    pub fn success(document: DidDocument, document_metadata: DocumentMetadata) -> Self {
        Self {
            document: Some(document),
            document_metadata: Some(document_metadata),
            resolution_metadata: ResolutionMetadata::default(),
        }
    }
    
    /// Create a failed resolution result
    pub fn error(error: &str, error_message: &str) -> Self {
        Self {
            document: None,
            document_metadata: None,
            resolution_metadata: ResolutionMetadata {
                error: Some(error.to_string()),
                error_message: Some(error_message.to_string()),
                ..Default::default()
            },
        }
    }
}

/// DID resolver trait
#[async_trait]
pub trait DidResolver: Send + Sync {
    /// Resolve a DID to a DID document
    async fn resolve(&self, did: &str) -> Result<ResolutionResult>;
    
    /// Resolve a DID to a DID document with additional parameters
    async fn resolve_with_params(&self, did: &str, params: &DidResolutionParams) -> Result<ResolutionResult>;
}

/// Parameters for DID resolution
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DidResolutionParams {
    /// Whether to accept a cached result
    pub accept_cached: bool,
    
    /// The version ID to resolve
    pub version_id: Option<String>,
    
    /// The version time to resolve
    pub version_time: Option<String>,
    
    /// Whether to resolve service endpoints
    pub resolve_services: bool,
}

impl Default for DidResolutionParams {
    fn default() -> Self {
        Self {
            accept_cached: true,
            version_id: None,
            version_time: None,
            resolve_services: true,
        }
    }
}

/// Local DID resolver implementation
#[derive(Debug)]
pub struct LocalDidResolver {
    /// Local DID document cache
    documents: RwLock<HashMap<String, DidDocument>>,
    
    /// Federation client for resolving remote DIDs
    federation_client: Arc<FederationClient>,
}

impl LocalDidResolver {
    /// Create a new local DID resolver without a federation client
    pub fn new() -> Self {
        Self {
            documents: RwLock::new(HashMap::new()),
            federation_client: Arc::new(FederationClient::default()),
        }
    }

    /// Create a clone of this resolver
    pub fn clone(&self) -> Self {
        let documents = self.documents.read().unwrap().clone();
        Self {
            documents: RwLock::new(documents),
            federation_client: self.federation_client.clone(),
        }
    }
    
    /// Store a DID document in the local cache
    pub fn store(&self, did: &str, document: DidDocument) -> Result<()> {
        let mut documents = self.documents.write().map_err(|_| Error::internal("Failed to acquire write lock on documents"))?;
        documents.insert(did.to_string(), document);
        Ok(())
    }
    
    /// Validate a DID
    fn validate_did(&self, did: &str) -> Result<(String, String, String)> {
        // Parse DID
        let parts: Vec<&str> = did.split(':').collect();
        
        // Validate DID format
        if parts.len() < 4 {
            return Err(Error::validation(format!("Invalid DID format: {}", did)));
        }
        
        if parts[0] != "did" {
            return Err(Error::validation(format!("Invalid DID scheme: {}", parts[0])));
        }
        
        if parts[1] != DID_METHOD {
            return Err(Error::validation(format!("Unsupported DID method: {}", parts[1])));
        }
        
        let federation_id = parts[2].to_string();
        let id = parts[3].to_string();
        
        Ok((did.to_string(), federation_id, id))
    }
}

#[async_trait]
impl DidResolver for LocalDidResolver {
    async fn resolve(&self, did: &str) -> Result<ResolutionResult> {
        self.resolve_with_params(did, &DidResolutionParams::default()).await
    }
    
    async fn resolve_with_params(&self, did: &str, params: &DidResolutionParams) -> Result<ResolutionResult> {
        // Validate DID format
        let (did, federation_id, _) = self.validate_did(did)?;
        
        // Check local cache first
        {
            let documents = self.documents.read().map_err(|_| Error::internal("Failed to acquire read lock on documents"))?;
            if let Some(document) = documents.get(&did) {
                // If we're allowed to return cached results, do so
                if params.accept_cached {
                    return Ok(ResolutionResult::success(
                        document.clone(),
                        DocumentMetadata::default(),
                    ));
                }
            }
        }
        
        // If not in local cache, try to resolve through the federation
        match self.federation_client.resolve_did(&did, &federation_id).await {
            Ok(document) => {
                // Store in local cache
                self.store(&did, document.clone())?;
                
                Ok(ResolutionResult::success(
                    document,
                    DocumentMetadata::default(),
                ))
            }
            Err(e) => {
                Ok(ResolutionResult::error(
                    "notFound",
                    &format!("Could not resolve DID: {}", e),
                ))
            }
        }
    }
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
                next_update: None,
                additional: HashMap::new(),
            }
        } else {
            // Create new metadata
            DocumentMetadata {
                created: Some(chrono::Utc::now().to_rfc3339()),
                updated: Some(chrono::Utc::now().to_rfc3339()),
                deactivated: None,
                version_id: Some("1".to_string()),
                next_update: None,
                additional: HashMap::new(),
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
                    if let crate::PublicKeyMaterial::Ed25519VerificationKey2020 { key } = &method.public_key {
                        if let Err(e) = bs58::decode(key).into_vec() {
                            return Err(Error::validation(
                                format!("Invalid Ed25519 public key: {}", e)
                            ));
                        }
                    } else {
                        return Err(Error::validation(
                            "Public key material type does not match verification method type"
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
                document: None,
                document_metadata: None,
                resolution_metadata: ResolutionMetadata {
                    error: Some("Invalid DID format".to_string()),
                    error_message: None,
                    content_type: None,
                    additional: HashMap::new(),
                },
            });
        }

        // Check if this DID belongs to our federation
        let did_federation = parts[2];
        if did_federation != federation_id {
            return Ok(ResolutionResult {
                document: None,
                document_metadata: None,
                resolution_metadata: ResolutionMetadata {
                    error: Some("DID not found in this federation".to_string()),
                    error_message: None,
                    content_type: None,
                    additional: HashMap::new(),
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
                document: None,
                document_metadata: None,
                resolution_metadata: ResolutionMetadata {
                    error: Some("invalidDid".to_string()),
                    error_message: None,
                    content_type: None,
                    additional: HashMap::new(),
                },
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
                document: Some(stored.document),
                document_metadata: Some(stored.metadata),
                resolution_metadata: ResolutionMetadata::default(),
            }),
            None => Ok(ResolutionResult {
                document: None,
                document_metadata: None,
                resolution_metadata: ResolutionMetadata {
                    error: Some("notFound".to_string()),
                    error_message: None,
                    content_type: None,
                    additional: HashMap::new(),
                },
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
    use crate::federation::MockFederationClient;
    
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
        assert!(result.document.is_some());
        assert_eq!(result.document.unwrap().id, did);
        
        // Update the document
        document.add_verification_method(crate::VerificationMethod {
            id: format!("{}#key-1", did),
            type_: "Ed25519VerificationKey2020".to_string(),
            controller: did.to_string(),
            public_key: crate::PublicKeyMaterial::Ed25519VerificationKey2020 {
                key: "2vSYXKMRQzuM5vPNZRyVdaZZzJBjRpbWqKxQDkZFHuMW".to_string() // Valid base58 string
            },
        });
        
        resolver.update(did, document.clone()).await.unwrap();
        
        // Resolve updated document
        let updated = resolver.resolve(did).await.unwrap();
        assert!(updated.document.is_some());
        assert_eq!(updated.document.unwrap().verification_method.len(), 1);
        
        // Test non-existent DID
        let not_found = resolver.resolve("did:icn:nonexistent").await.unwrap();
        assert!(not_found.document.is_none());
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
        assert!(result.document.is_none());
        assert_eq!(
            result.resolution_metadata.error.unwrap(),
            "invalidDid"
        );
        
        // Test non-existent DID
        let result = resolver.resolve("did:icn:nonexistent").await.unwrap();
        assert!(result.document.is_none());
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
        assert!(result.document.is_some());
        assert_eq!(result.document.unwrap().id, doc.id);

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
        assert!(result.document.is_some());
        let resolved_doc = result.document.unwrap();
        assert_eq!(resolved_doc.service.len(), 1);
        assert_eq!(resolved_doc.service[0].type_, "TestService");

        // Deactivate the document
        resolver.deactivate(did).await.unwrap();

        // Resolve deactivated document
        let result = resolver.resolve(did).await.unwrap();
        assert!(result.document.is_some());
        assert!(result.document_metadata.as_ref().unwrap().deactivated.unwrap());
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
            
        assert!(resolution.document.is_none());
        assert!(resolution.resolution_metadata.error.is_some());
        
        // Test with different federation DID
        let did = "did:icn:other-fed:123";
        let resolution = resolver
            .handle_federation_resolution(did, "test-fed")
            .await
            .unwrap();
            
        assert!(resolution.document.is_none());
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
            
        assert!(resolution.document.is_none());
        assert_eq!(
            resolution.resolution_metadata.error.unwrap(),
            "Invalid DID format"
        );
    }

    #[tokio::test]
    async fn test_local_resolution() {
        // Create a mock federation client
        let federation_client = Arc::new(MockFederationClient::new());
        
        // Create resolver
        let resolver = LocalDidResolver::new();
        
        // Create a test DID document
        let did = "did:icn:test:123";
        let document = DidDocument::new(did).unwrap();
        
        // Store it
        resolver.store(did, document.clone()).unwrap();
        
        // Resolve it
        let result = resolver.resolve(did).await.unwrap();
        
        // Should succeed
        assert!(result.document.is_some());
        assert_eq!(result.document.unwrap().id, did);
    }
    
    #[test]
    fn test_validate_did() {
        // Create a mock federation client
        let federation_client = Arc::new(MockFederationClient::new());
        
        // Create resolver
        let resolver = LocalDidResolver::new();
        
        // Valid DID
        let (did, federation_id, id) = resolver.validate_did("did:icn:test:123").unwrap();
        assert_eq!(did, "did:icn:test:123");
        assert_eq!(federation_id, "test");
        assert_eq!(id, "123");
        
        // Invalid scheme
        assert!(resolver.validate_did("foo:icn:test:123").is_err());
        
        // Invalid method
        assert!(resolver.validate_did("did:foo:test:123").is_err());
        
        // Too few parts
        assert!(resolver.validate_did("did:icn").is_err());
    }
}