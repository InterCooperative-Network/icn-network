//! Federation client for cross-federation DID operations
use async_trait::async_trait;
use icn_common::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::{DidDocument, DID_METHOD};
use crate::resolver::{DidResolver, ResolutionResult, DocumentMetadata, ResolutionMetadata};
use icn_crypto::Signature;
use reqwest::Client;
use std::time::{Duration, Instant};
use uuid::Uuid;

/// Federation client trait
#[async_trait]
pub trait FederationClient: Send + Sync {
    /// Resolve a DID through a federation
    async fn resolve_did(&self, did: &str, federation_id: &str) -> Result<DidDocument>;
    
    /// Register a DID document with a federation
    async fn register_did(&self, document: &DidDocument, federation_id: &str) -> Result<()>;
    
    /// Update a DID document in a federation
    async fn update_did(&self, document: &DidDocument, federation_id: &str) -> Result<()>;
    
    /// Deactivate a DID in a federation
    async fn deactivate_did(&self, did: &str, federation_id: &str) -> Result<()>;
}

/// Basic federation client implementation
#[derive(Debug)]
pub struct BasicFederationClient {
    /// Federation endpoints
    endpoints: HashMap<String, String>,
    
    /// Client for making HTTP requests
    client: reqwest::Client,
}

impl BasicFederationClient {
    /// Create a new federation client
    pub fn new(endpoints: HashMap<String, String>) -> Self {
        Self {
            endpoints,
            client: reqwest::Client::new(),
        }
    }
    
    /// Get the endpoint for a federation
    fn get_endpoint(&self, federation_id: &str) -> Result<String> {
        self.endpoints
            .get(federation_id)
            .cloned()
            .ok_or_else(|| Error::not_found(format!("Federation endpoint not found: {}", federation_id)))
    }
}

impl Default for BasicFederationClient {
    fn default() -> Self {
        Self {
            endpoints: HashMap::new(),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl FederationClient for BasicFederationClient {
    async fn resolve_did(&self, did: &str, federation_id: &str) -> Result<DidDocument> {
        let endpoint = self.get_endpoint(federation_id)?;
        
        // Construct the resolve URL
        let url = format!("{}/v1/identities/{}", endpoint, did);
        
        // Make the request
        let response = self.client.get(&url)
            .send()
            .await
            .map_err(|e| Error::external(format!("Failed to resolve DID: {}", e)))?;
        
        // Check the response
        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await
                .map_err(|e| Error::external(format!("Failed to read error response: {}", e)))?;
            
            return Err(Error::external(format!("Failed to resolve DID: {} - {}", status, body)));
        }
        
        // Parse the response
        let document: DidDocument = response.json()
            .await
            .map_err(|e| Error::external(format!("Failed to parse DID document: {}", e)))?;
        
        Ok(document)
    }
    
    async fn register_did(&self, document: &DidDocument, federation_id: &str) -> Result<()> {
        let endpoint = self.get_endpoint(federation_id)?;
        
        // Construct the register URL
        let url = format!("{}/v1/identities", endpoint);
        
        // Make the request
        let response = self.client.post(&url)
            .json(document)
            .send()
            .await
            .map_err(|e| Error::external(format!("Failed to register DID: {}", e)))?;
        
        // Check the response
        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await
                .map_err(|e| Error::external(format!("Failed to read error response: {}", e)))?;
            
            return Err(Error::external(format!("Failed to register DID: {} - {}", status, body)));
        }
        
        Ok(())
    }
    
    async fn update_did(&self, document: &DidDocument, federation_id: &str) -> Result<()> {
        let endpoint = self.get_endpoint(federation_id)?;
        
        // Construct the update URL
        let url = format!("{}/v1/identities/{}", endpoint, document.id);
        
        // Make the request
        let response = self.client.put(&url)
            .json(document)
            .send()
            .await
            .map_err(|e| Error::external(format!("Failed to update DID: {}", e)))?;
        
        // Check the response
        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await
                .map_err(|e| Error::external(format!("Failed to read error response: {}", e)))?;
            
            return Err(Error::external(format!("Failed to update DID: {} - {}", status, body)));
        }
        
        Ok(())
    }
    
    async fn deactivate_did(&self, did: &str, federation_id: &str) -> Result<()> {
        let endpoint = self.get_endpoint(federation_id)?;
        
        // Construct the deactivate URL
        let url = format!("{}/v1/identities/{}/deactivate", endpoint, did);
        
        // Make the request
        let response = self.client.post(&url)
            .send()
            .await
            .map_err(|e| Error::external(format!("Failed to deactivate DID: {}", e)))?;
        
        // Check the response
        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await
                .map_err(|e| Error::external(format!("Failed to read error response: {}", e)))?;
            
            return Err(Error::external(format!("Failed to deactivate DID: {} - {}", status, body)));
        }
        
        Ok(())
    }
}

/// A mock federation client for testing
#[derive(Debug)]
pub struct MockFederationClient {
    /// Mock documents
    documents: RwLock<HashMap<String, DidDocument>>,
}

impl MockFederationClient {
    /// Create a new mock federation client
    pub fn new() -> Self {
        Self {
            documents: RwLock::new(HashMap::new()),
        }
    }
    
    /// Set a document for testing
    pub fn set_document(&self, did: &str, document: DidDocument) -> Result<()> {
        let mut documents = self.documents.write().map_err(|_| Error::internal("Failed to acquire write lock on documents"))?;
        documents.insert(did.to_string(), document);
        Ok(())
    }
}

#[async_trait]
impl FederationClient for MockFederationClient {
    async fn resolve_did(&self, did: &str, _federation_id: &str) -> Result<DidDocument> {
        let documents = self.documents.read().map_err(|_| Error::internal("Failed to acquire read lock on documents"))?;
        
        documents.get(did)
            .cloned()
            .ok_or_else(|| Error::not_found(format!("DID not found: {}", did)))
    }
    
    async fn register_did(&self, document: &DidDocument, _federation_id: &str) -> Result<()> {
        let did = document.id.clone();
        self.set_document(&did, document.clone())
    }
    
    async fn update_did(&self, document: &DidDocument, _federation_id: &str) -> Result<()> {
        let did = document.id.clone();
        self.set_document(&did, document.clone())
    }
    
    async fn deactivate_did(&self, did: &str, _federation_id: &str) -> Result<()> {
        // In a real client, this would mark the DID as deactivated
        // For now, let's just remove it
        let mut documents = self.documents.write().map_err(|_| Error::internal("Failed to acquire write lock on documents"))?;
        
        if documents.remove(did).is_none() {
            return Err(Error::not_found(format!("DID not found: {}", did)));
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::verification::PublicKeyMaterial;
    use std::time::Duration;

    #[tokio::test]
    async fn test_federation_client_creation() {
        let client = FederationClient::new(
            "local-fed",
            vec!["http://localhost:8080".to_string()]
        ).await.unwrap();
        
        assert_eq!(client.federation_id(), "local-fed");
        assert_eq!(client.endpoints.len(), 1);
    }
    
    #[tokio::test]
    async fn test_federation_resolver() {
        let client = FederationClient::new(
            "local-fed",
            vec!["http://localhost:8080".to_string()]
        ).await.unwrap();
        let resolver = FederationDidResolver::new(client);
        
        // Test federation extraction
        let federation_id = resolver.extract_federation_id("did:icn:external-fed:abc123");
        assert_eq!(federation_id, Some("external-fed".to_string()));
        
        // Test invalid DID format
        let federation_id = resolver.extract_federation_id("did:example:invalid");
        assert_eq!(federation_id, None);
    }
}