use async_trait::async_trait;
use icn_common::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::{DidDocument, DID_METHOD};

/// Federation client for cross-federation DID operations
pub struct FederationClient {
    /// Federation ID for this node
    federation_id: String,
    
    /// Federation endpoints
    endpoints: Vec<String>,
    
    /// Cached DID documents from other federations
    cache: Arc<RwLock<HashMap<String, CachedDocument>>>,
}

/// A cached DID document with metadata
#[derive(Clone, Debug)]
struct CachedDocument {
    document: DidDocument,
    timestamp: u64,
    ttl: u64,
}

/// Federation resolution request
#[derive(Debug, Serialize, Deserialize)]
struct ResolutionRequest {
    did: String,
    federation_id: String,
}

/// Federation resolution response
#[derive(Debug, Serialize, Deserialize)]
struct ResolutionResponse {
    document: Option<DidDocument>,
    error: Option<String>,
}

impl FederationClient {
    /// Create a new federation client
    pub async fn new(federation_id: &str, endpoints: Vec<String>) -> Result<Self> {
        Ok(Self {
            federation_id: federation_id.to_string(),
            endpoints,
            cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Resolve a DID from another federation
    pub async fn resolve_did(&self, did: &str, federation_id: &str) -> Result<Option<DidDocument>> {
        // Check cache first
        if let Some(cached) = self.get_cached_document(did).await? {
            return Ok(Some(cached.document));
        }

        // If not in cache, try federation resolution
        for endpoint in &self.endpoints {
            match self.resolve_from_endpoint(endpoint, did, federation_id).await {
                Ok(Some(doc)) => {
                    // Cache the document
                    self.cache_document(did, doc.clone(), 3600).await?;
                    return Ok(Some(doc));
                }
                Ok(None) => continue,
                Err(e) => {
                    log::warn!("Failed to resolve DID from endpoint {}: {}", endpoint, e);
                    continue;
                }
            }
        }

        Ok(None)
    }

    /// Get a cached document if it exists and hasn't expired
    async fn get_cached_document(&self, did: &str) -> Result<Option<CachedDocument>> {
        let cache = self.cache.read().await;
        if let Some(cached) = cache.get(did) {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|e| Error::system_time("Failed to get system time"))?
                .as_secs();

            if now - cached.timestamp < cached.ttl {
                return Ok(Some(cached.clone()));
            }
        }
        Ok(None)
    }

    /// Cache a document with TTL
    async fn cache_document(&self, did: &str, document: DidDocument, ttl: u64) -> Result<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| Error::system_time("Failed to get system time"))?
            .as_secs();

        let cached = CachedDocument {
            document,
            timestamp: now,
            ttl,
        };

        let mut cache = self.cache.write().await;
        cache.insert(did.to_string(), cached);
        Ok(())
    }

    /// Resolve a DID from a specific federation endpoint
    async fn resolve_from_endpoint(
        &self,
        endpoint: &str,
        did: &str,
        federation_id: &str,
    ) -> Result<Option<DidDocument>> {
        let client = reqwest::Client::new();
        
        let request = ResolutionRequest {
            did: did.to_string(),
            federation_id: federation_id.to_string(),
        };

        let response = client
            .post(endpoint)
            .json(&request)
            .send()
            .await
            .map_err(|e| Error::network(format!("Failed to send resolution request: {}", e)))?;

        let resolution: ResolutionResponse = response
            .json()
            .await
            .map_err(|e| Error::network(format!("Failed to parse resolution response: {}", e)))?;

        if let Some(error) = resolution.error {
            return Err(Error::resolution(error));
        }

        Ok(resolution.document)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::verification::PublicKeyMaterial;
    use std::time::Duration;

    #[tokio::test]
    async fn test_federation_client() {
        let client = FederationClient::new(
            "test-federation",
            vec!["http://localhost:8080".to_string()]
        ).await.unwrap();

        // Test caching
        let doc = DidDocument::new("test123").unwrap();
        client.cache_document(&doc.id, doc.clone(), 1).await.unwrap();

        // Should get from cache
        let cached = client.get_cached_document(&doc.id).await.unwrap();
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().document.id, doc.id);

        // Wait for TTL to expire
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Should not get from cache
        let cached = client.get_cached_document(&doc.id).await.unwrap();
        assert!(cached.is_none());
    }

    #[tokio::test]
    async fn test_federation_resolution() {
        let client = FederationClient::new(
            "test-federation",
            vec!["http://localhost:8080".to_string()]
        ).await.unwrap();

        // Test resolution with invalid endpoint (should handle error gracefully)
        let result = client
            .resolve_did("did:icn:test:123", "other-federation")
            .await
            .unwrap();
        assert!(result.is_none());
    }
}