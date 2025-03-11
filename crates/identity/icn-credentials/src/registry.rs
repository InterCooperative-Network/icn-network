//! Registry types and functionality for the ICN verifiable credentials system
//!
//! This module provides the structures and functions for storing, looking up,
//! and managing verifiable credentials.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use icn_common::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::VerifiableCredential;

/// Options for configuring a credential registry
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RegistryOptions {
    /// Whether to validate credentials when adding them to the registry
    pub validate_on_add: bool,
    
    /// Whether to check for duplicates when adding credentials
    pub check_duplicates: bool,
    
    /// Whether to automatically remove expired credentials
    pub auto_remove_expired: bool,
    
    /// Any additional options specific to the registry implementation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_options: Option<serde_json::Value>,
}

impl Default for RegistryOptions {
    fn default() -> Self {
        RegistryOptions {
            validate_on_add: true,
            check_duplicates: true,
            auto_remove_expired: true,
            additional_options: None,
        }
    }
}

/// Query for looking up credentials in a registry
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CredentialQuery {
    /// Optional issuer DID to filter by
    pub issuer: Option<String>,
    
    /// Optional subject DID to filter by
    pub subject: Option<String>,
    
    /// Optional credential types to filter by (any match)
    pub types: Option<Vec<String>>,
    
    /// Optional property queries (all must match)
    pub properties: Option<HashMap<String, serde_json::Value>>,
    
    /// Optional time range for issuance date
    pub issued_after: Option<DateTime<Utc>>,
    pub issued_before: Option<DateTime<Utc>>,
    
    /// Optional query for non-expired credentials
    pub not_expired: Option<bool>,
    
    /// Optional limit on the number of results
    pub limit: Option<usize>,
    
    /// Optional pagination offset
    pub offset: Option<usize>,
}

impl CredentialQuery {
    /// Create a new empty query
    pub fn new() -> Self {
        CredentialQuery {
            issuer: None,
            subject: None,
            types: None,
            properties: None,
            issued_after: None,
            issued_before: None,
            not_expired: None,
            limit: None,
            offset: None,
        }
    }
    
    /// Set the issuer filter
    pub fn with_issuer(mut self, issuer: &str) -> Self {
        self.issuer = Some(issuer.to_string());
        self
    }
    
    /// Set the subject filter
    pub fn with_subject(mut self, subject: &str) -> Self {
        self.subject = Some(subject.to_string());
        self
    }
    
    /// Add a type filter
    pub fn with_type(mut self, type_: &str) -> Self {
        let mut types = self.types.unwrap_or_else(Vec::new);
        types.push(type_.to_string());
        self.types = Some(types);
        self
    }
    
    /// Add a property filter
    pub fn with_property<T: Into<serde_json::Value>>(mut self, name: &str, value: T) -> Self {
        let mut properties = self.properties.unwrap_or_else(HashMap::new);
        properties.insert(name.to_string(), value.into());
        self.properties = Some(properties);
        self
    }
    
    /// Set the issued after filter
    pub fn issued_after(mut self, date: DateTime<Utc>) -> Self {
        self.issued_after = Some(date);
        self
    }
    
    /// Set the issued before filter
    pub fn issued_before(mut self, date: DateTime<Utc>) -> Self {
        self.issued_before = Some(date);
        self
    }
    
    /// Set the not expired filter
    pub fn not_expired(mut self, value: bool) -> Self {
        self.not_expired = Some(value);
        self
    }
    
    /// Set the limit
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
    
    /// Set the offset
    pub fn offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }
}

/// Registry interface for storing and looking up verifiable credentials
#[async_trait]
pub trait CredentialRegistry {
    /// Add a credential to the registry
    async fn add_credential(&self, credential: VerifiableCredential) -> Result<bool>;
    
    /// Get a credential by ID
    async fn get_credential(&self, id: &str) -> Result<Option<VerifiableCredential>>;
    
    /// Query credentials by various criteria
    async fn query_credentials(&self, query: &CredentialQuery) -> Result<Vec<VerifiableCredential>>;
    
    /// Remove a credential from the registry
    async fn remove_credential(&self, id: &str) -> Result<bool>;
    
    /// Check if a credential exists in the registry
    async fn has_credential(&self, id: &str) -> Result<bool>;
    
    /// Count credentials matching a query
    async fn count_credentials(&self, query: &CredentialQuery) -> Result<usize>;
}

/// In-memory implementation of CredentialRegistry
pub struct InMemoryRegistry {
    credentials: Arc<RwLock<HashMap<String, VerifiableCredential>>>,
    options: RegistryOptions,
}

impl InMemoryRegistry {
    /// Create a new in-memory registry
    pub fn new(options: RegistryOptions) -> Self {
        InMemoryRegistry {
            credentials: Arc::new(RwLock::new(HashMap::new())),
            options,
        }
    }
    
    /// Create a new in-memory registry with default options
    pub fn default() -> Self {
        Self::new(RegistryOptions::default())
    }
    
    /// Check if a credential matches a query
    fn matches_query(&self, credential: &VerifiableCredential, query: &CredentialQuery) -> bool {
        // Check issuer
        if let Some(issuer) = &query.issuer {
            if credential.issuer != *issuer {
                return false;
            }
        }
        
        // Check subject
        if let Some(subject) = &query.subject {
            if credential.credential_subject.id.as_ref() != Some(subject) {
                return false;
            }
        }
        
        // Check types
        if let Some(types) = &query.types {
            if !types.iter().any(|t| credential.types.contains(t)) {
                return false;
            }
        }
        
        // Check properties
        if let Some(properties) = &query.properties {
            for (name, value) in properties {
                if !credential.credential_subject.properties.get(name)
                    .map(|v| v == value)
                    .unwrap_or(false) {
                    return false;
                }
            }
        }
        
        // Check issuance date range
        if let Some(after) = query.issued_after {
            if credential.issuance_date < after {
                return false;
            }
        }
        
        if let Some(before) = query.issued_before {
            if credential.issuance_date > before {
                return false;
            }
        }
        
        // Check expiration
        if let Some(true) = query.not_expired {
            if let Some(expiration) = credential.expiration_date {
                if expiration < Utc::now() {
                    return false;
                }
            }
        }
        
        true
    }
}

#[async_trait]
impl CredentialRegistry for InMemoryRegistry {
    async fn add_credential(&self, credential: VerifiableCredential) -> Result<bool> {
        // Check if we should validate on add
        if self.options.validate_on_add {
            // Validation would happen here - for now, just check if there's a proof
            if credential.proof.is_none() {
                return Ok(false);
            }
        }
        
        // Check if we should verify duplicates
        if self.options.check_duplicates {
            let read_guard = self.credentials.read().unwrap();
            if read_guard.contains_key(&credential.id) {
                return Ok(false);
            }
        }
        
        // Check if the credential is expired and we auto-remove expired
        if self.options.auto_remove_expired {
            if let Some(expiration) = credential.expiration_date {
                if expiration < Utc::now() {
                    return Ok(false);
                }
            }
        }
        
        // Add the credential
        let mut write_guard = self.credentials.write().unwrap();
        write_guard.insert(credential.id.clone(), credential);
        
        Ok(true)
    }
    
    async fn get_credential(&self, id: &str) -> Result<Option<VerifiableCredential>> {
        let read_guard = self.credentials.read().unwrap();
        Ok(read_guard.get(id).cloned())
    }
    
    async fn query_credentials(&self, query: &CredentialQuery) -> Result<Vec<VerifiableCredential>> {
        let read_guard = self.credentials.read().unwrap();
        
        // Filter credentials by the query
        let mut results: Vec<VerifiableCredential> = read_guard.values()
            .filter(|cred| self.matches_query(cred, query))
            .cloned()
            .collect();
        
        // Apply offset and limit if specified
        if let Some(offset) = query.offset {
            results = results.into_iter().skip(offset).collect();
        }
        
        if let Some(limit) = query.limit {
            results = results.into_iter().take(limit).collect();
        }
        
        Ok(results)
    }
    
    async fn remove_credential(&self, id: &str) -> Result<bool> {
        let mut write_guard = self.credentials.write().unwrap();
        Ok(write_guard.remove(id).is_some())
    }
    
    async fn has_credential(&self, id: &str) -> Result<bool> {
        let read_guard = self.credentials.read().unwrap();
        Ok(read_guard.contains_key(id))
    }
    
    async fn count_credentials(&self, query: &CredentialQuery) -> Result<usize> {
        let read_guard = self.credentials.read().unwrap();
        let count = read_guard.values()
            .filter(|cred| self.matches_query(cred, query))
            .count();
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CredentialSubject, Proof, ProofPurpose, ProofType};
    use icn_crypto::signature::Signature;
    
    #[tokio::test]
    async fn test_in_memory_registry() {
        // Create a registry
        let registry = InMemoryRegistry::default();
        
        // Create a test credential
        let mut subject = CredentialSubject::new(Some("did:icn:test:subject".to_string()));
        subject.add_property("name", "Alice");
        subject.add_property("membershipLevel", "Gold");
        
        let credential = VerifiableCredential {
            context: vec!["https://www.w3.org/2018/credentials/v1".to_string()],
            id: "test-credential-1".to_string(),
            types: vec!["VerifiableCredential".to_string(), "MembershipCredential".to_string()],
            issuer: "did:icn:test:issuer".to_string(),
            issuance_date: Utc::now(),
            expiration_date: Some(Utc::now() + chrono::Duration::days(365)),
            credential_subject: subject,
            credential_status: None,
            credential_schema: None,
            refresh_service: None,
            terms_of_use: None,
            evidence: None,
            proof: Some(Proof {
                type_: ProofType::Ed25519Signature2020,
                created: Utc::now(),
                verification_method: "did:icn:test:issuer#keys-1".to_string(),
                purpose: ProofPurpose::AssertionMethod,
                value: Signature::Ed25519(vec![1, 2, 3, 4]), // Dummy signature for testing
                jws: None,
                domain: None,
                challenge: None,
                nonce: None,
            }),
        };
        
        // Add the credential to the registry
        let result = registry.add_credential(credential.clone()).await.unwrap();
        assert!(result);
        
        // Get the credential by ID
        let retrieved = registry.get_credential(&credential.id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, credential.id);
        
        // Query by issuer
        let query = CredentialQuery::new().with_issuer("did:icn:test:issuer");
        let results = registry.query_credentials(&query).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, credential.id);
        
        // Query by subject
        let query = CredentialQuery::new().with_subject("did:icn:test:subject");
        let results = registry.query_credentials(&query).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, credential.id);
        
        // Query by type
        let query = CredentialQuery::new().with_type("MembershipCredential");
        let results = registry.query_credentials(&query).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, credential.id);
        
        // Query by property
        let query = CredentialQuery::new().with_property("membershipLevel", "Gold");
        let results = registry.query_credentials(&query).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, credential.id);
        
        // Remove the credential
        let result = registry.remove_credential(&credential.id).await.unwrap();
        assert!(result);
        
        // Check that it's gone
        let retrieved = registry.get_credential(&credential.id).await.unwrap();
        assert!(retrieved.is_none());
    }
} 