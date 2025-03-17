//! Credential-based storage system for ICN
//!
//! This module extends the identity-integrated storage with support for 
//! verifiable credentials, allowing attribute-based access control.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::SystemTime;
use tokio::fs;
use tracing::{debug, info, warn};

use crate::identity_storage::{IdentityProvider, IdentityStorageService, DidVerificationStatus, DidDocument};

/// Credential verification status
#[derive(Debug, Clone, PartialEq)]
pub enum CredentialVerificationStatus {
    /// Credential verification succeeded
    Verified,
    /// Credential verification failed
    Failed,
    /// Credential not found
    NotFound,
    /// Credential revoked
    Revoked,
    /// Credential expired
    Expired,
    /// Credential verification error
    Error(String),
}

/// Verifiable credential simplified for storage integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifiableCredential {
    /// Context defines the JSON-LD schema
    #[serde(rename = "@context")]
    pub context: Vec<String>,
    /// ID of this credential
    pub id: String,
    /// Type of credential
    #[serde(rename = "type")]
    pub credential_type: Vec<String>,
    /// Credential issuer
    pub issuer: String,
    /// Issuance date (ISO 8601)
    pub issuanceDate: String,
    /// Expiration date (ISO 8601)
    pub expirationDate: Option<String>,
    /// Credential subject containing the claims
    pub credentialSubject: CredentialSubject,
    /// Credential proof
    pub proof: CredentialProof,
    /// Revocation status
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revocation: Option<RevocationInfo>,
}

/// Credential subject containing claims
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialSubject {
    /// Subject identifier (DID)
    pub id: String,
    /// Claims (attributes) in the credential
    #[serde(flatten)]
    pub claims: serde_json::Map<String, serde_json::Value>,
}

/// Credential proof
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialProof {
    /// Type of proof
    #[serde(rename = "type")]
    pub proof_type: String,
    /// Creation date of proof
    pub created: String,
    /// Verification method used
    pub verificationMethod: String,
    /// Purpose of the proof
    pub proofPurpose: String,
    /// Signature value
    #[serde(rename = "jws", skip_serializing_if = "Option::is_none")]
    pub jws: Option<String>,
    /// Multibase signature value (alternative to JWS)
    #[serde(rename = "proofValue", skip_serializing_if = "Option::is_none")]
    pub proof_value: Option<String>,
}

/// Revocation information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevocationInfo {
    /// Revocation date
    pub date: String,
    /// Revocation reason
    pub reason: Option<String>,
    /// Revocation authority
    pub authority: String,
}

/// Credential provider for verifiable credential resolution and verification
pub trait CredentialProvider {
    /// Resolve a credential by ID
    async fn resolve_credential(&self, credential_id: &str) -> Result<Option<VerifiableCredential>>;
    
    /// Verify a credential's validity and authenticity
    async fn verify_credential(&self, credential: &VerifiableCredential) -> Result<CredentialVerificationStatus>;
    
    /// Check if a credential has specific attributes matching required values
    async fn check_credential_attributes(
        &self, 
        credential: &VerifiableCredential, 
        required_attributes: &HashMap<String, serde_json::Value>
    ) -> Result<bool>;
    
    /// Get all credentials for a subject (DID)
    async fn get_credentials_for_subject(&self, subject_id: &str) -> Result<Vec<VerifiableCredential>>;
}

/// Mock credential provider for testing
#[derive(Default)]
pub struct MockCredentialProvider {
    /// Mock credentials
    credentials: HashMap<String, VerifiableCredential>,
    /// Subject to credentials mapping
    subject_credentials: HashMap<String, Vec<String>>,
}

impl MockCredentialProvider {
    /// Create a new mock credential provider
    pub fn new() -> Self {
        Self {
            credentials: HashMap::new(),
            subject_credentials: HashMap::new(),
        }
    }
    
    /// Add a mock credential
    pub fn add_credential(&mut self, credential: VerifiableCredential) {
        let subject_id = credential.credentialSubject.id.clone();
        let credential_id = credential.id.clone();
        
        // Add to subject mapping
        self.subject_credentials
            .entry(subject_id)
            .or_insert_with(Vec::new)
            .push(credential_id.clone());
        
        // Add to credentials map
        self.credentials.insert(credential_id, credential);
    }
}

impl CredentialProvider for MockCredentialProvider {
    async fn resolve_credential(&self, credential_id: &str) -> Result<Option<VerifiableCredential>> {
        Ok(self.credentials.get(credential_id).cloned())
    }
    
    async fn verify_credential(&self, credential: &VerifiableCredential) -> Result<CredentialVerificationStatus> {
        // Check if credential exists
        if !self.credentials.contains_key(&credential.id) {
            return Ok(CredentialVerificationStatus::NotFound);
        }
        
        // Check if revoked
        if let Some(revocation) = &credential.revocation {
            return Ok(CredentialVerificationStatus::Revoked);
        }
        
        // Check expiration
        if let Some(expiration_date) = &credential.expirationDate {
            // Parse expiration date
            let expiration = chrono::DateTime::parse_from_rfc3339(expiration_date)
                .map_err(|e| anyhow!("Failed to parse expiration date: {}", e))?;
            
            // Check if expired
            let now = chrono::Utc::now();
            if now > expiration.with_timezone(&chrono::Utc) {
                return Ok(CredentialVerificationStatus::Expired);
            }
        }
        
        // For a mock provider, we'll assume the credential is valid if it exists and isn't revoked or expired
        Ok(CredentialVerificationStatus::Verified)
    }
    
    async fn check_credential_attributes(
        &self, 
        credential: &VerifiableCredential, 
        required_attributes: &HashMap<String, serde_json::Value>
    ) -> Result<bool> {
        // First, check that the credential is valid
        let status = self.verify_credential(credential).await?;
        if status != CredentialVerificationStatus::Verified {
            return Ok(false);
        }
        
        // Check all required attributes
        for (key, required_value) in required_attributes {
            // Check if key exists in claims
            if !credential.credentialSubject.claims.contains_key(key) {
                return Ok(false);
            }
            
            // Get actual value
            let actual_value = &credential.credentialSubject.claims[key];
            
            // Compare values (basic equality check)
            if actual_value != required_value {
                return Ok(false);
            }
        }
        
        // All attributes matched
        Ok(true)
    }
    
    async fn get_credentials_for_subject(&self, subject_id: &str) -> Result<Vec<VerifiableCredential>> {
        // Get credential IDs for this subject
        let credential_ids = match self.subject_credentials.get(subject_id) {
            Some(ids) => ids,
            None => return Ok(Vec::new()),
        };
        
        // Collect credentials
        let mut credentials = Vec::new();
        for id in credential_ids {
            if let Some(cred) = self.credentials.get(id) {
                credentials.push(cred.clone());
            }
        }
        
        Ok(credentials)
    }
}

/// Policy rule for credential-based access control
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialAccessRule {
    /// File pattern this rule applies to
    pub pattern: String,
    /// Required credential types (at least one must match)
    pub credential_types: Vec<String>,
    /// Required credential attributes (all must match)
    pub required_attributes: HashMap<String, serde_json::Value>,
    /// Permissions granted if credential requirements are met
    pub permissions: Vec<String>,
}

/// Credential-based storage service extending identity storage
pub struct CredentialStorageService<I: IdentityProvider, C: CredentialProvider> {
    /// Identity storage service
    identity_storage: IdentityStorageService<I>,
    /// Credential provider
    credential_provider: C,
    /// Access rules - credential-based policies
    access_rules: Vec<CredentialAccessRule>,
}

impl<I: IdentityProvider, C: CredentialProvider> CredentialStorageService<I, C> {
    /// Create a new credential storage service
    pub async fn new(
        federation: &str,
        data_path: impl Into<PathBuf>,
        identity_provider: I,
        credential_provider: C,
        cache_ttl: u64,
    ) -> Result<Self> {
        let identity_storage = IdentityStorageService::new(
            federation,
            data_path,
            identity_provider,
            cache_ttl,
        ).await?;
        
        Ok(Self {
            identity_storage,
            credential_provider,
            access_rules: Vec::new(),
        })
    }
    
    /// Load credential access rules from storage
    pub async fn load_access_rules(&mut self, rules_path: impl AsRef<std::path::Path>) -> Result<()> {
        let content = fs::read_to_string(rules_path).await?;
        let rules: Vec<CredentialAccessRule> = serde_json::from_str(&content)?;
        self.access_rules = rules;
        Ok(())
    }
    
    /// Save credential access rules to storage
    pub async fn save_access_rules(&self, rules_path: impl AsRef<std::path::Path>) -> Result<()> {
        let content = serde_json::to_string_pretty(&self.access_rules)?;
        fs::write(rules_path, content).await?;
        Ok(())
    }
    
    /// Add a credential access rule
    pub fn add_access_rule(&mut self, rule: CredentialAccessRule) {
        self.access_rules.push(rule);
    }
    
    /// Check if a credential grants permissions for a specific file
    async fn check_credential_permissions(
        &self,
        credential: &VerifiableCredential,
        file_key: &str,
        required_permission: &str,
    ) -> Result<bool> {
        for rule in &self.access_rules {
            // Check if rule pattern matches the file key
            if !glob_match::glob_match(&rule.pattern, file_key) {
                continue;
            }
            
            // Check if credential type is in the required types
            let type_match = credential.credential_type.iter()
                .any(|t| rule.credential_types.contains(t));
            
            if !type_match {
                continue;
            }
            
            // Check if credential has required attributes
            let attr_match = self.credential_provider
                .check_credential_attributes(credential, &rule.required_attributes)
                .await?;
            
            if !attr_match {
                continue;
            }
            
            // Check if requested permission is granted by this rule
            if rule.permissions.contains(&required_permission.to_string()) {
                return Ok(true);
            }
        }
        
        // No matching rule found
        Ok(false)
    }
    
    /// Store a file with DID and credential-based authentication
    pub async fn store_file(
        &mut self,
        did: &str,
        challenge: &[u8],
        signature: &[u8],
        credential_id: Option<&str>,
        file_path: impl AsRef<std::path::Path>,
        key: &str,
        encrypted: bool,
    ) -> Result<()> {
        // First, authenticate the DID
        let member_id = self.authenticate_and_get_member_id(did, challenge, signature).await?;
        
        // If a credential ID is provided, check it for write permission
        if let Some(cred_id) = credential_id {
            let has_permission = self.check_credential_permission(
                cred_id, 
                did,
                key, 
                "write"
            ).await?;
            
            if !has_permission {
                return Err(anyhow!("Credential doesn't grant write permission to this file"));
            }
        }
        
        // Use the identity storage to store the file
        self.identity_storage.store_file(
            did,
            challenge,
            signature,
            file_path,
            key,
            encrypted,
        ).await
    }
    
    /// Retrieve a file with DID and credential-based authentication
    pub async fn retrieve_file(
        &mut self,
        did: &str,
        challenge: &[u8],
        signature: &[u8],
        credential_id: Option<&str>,
        key: &str,
        output_path: impl AsRef<std::path::Path>,
        version: Option<&str>,
    ) -> Result<()> {
        // First, authenticate the DID
        let member_id = self.authenticate_and_get_member_id(did, challenge, signature).await?;
        
        // If a credential ID is provided, check it for read permission
        if let Some(cred_id) = credential_id {
            let has_permission = self.check_credential_permission(
                cred_id, 
                did,
                key, 
                "read"
            ).await?;
            
            if !has_permission {
                return Err(anyhow!("Credential doesn't grant read permission to this file"));
            }
        }
        
        // Use the identity storage to retrieve the file
        self.identity_storage.retrieve_file(
            did,
            challenge,
            signature,
            key,
            output_path,
            version,
        ).await
    }
    
    /// List files accessible with DID and credential-based authentication
    pub async fn list_files(
        &mut self,
        did: &str,
        challenge: &[u8],
        signature: &[u8],
        credential_id: Option<&str>,
        prefix: Option<&str>,
    ) -> Result<Vec<crate::storage::VersionedFileMetadata>> {
        // First, authenticate the DID
        let member_id = self.authenticate_and_get_member_id(did, challenge, signature).await?;
        
        // Use identity storage to list all potentially accessible files
        let all_files = self.identity_storage.list_files(
            did,
            challenge,
            signature,
            prefix,
        ).await?;
        
        // If no credential provided, return the standard authorized files
        if credential_id.is_none() {
            return Ok(all_files);
        }
        
        // Otherwise, filter files based on credential permissions
        let cred_id = credential_id.unwrap();
        let mut accessible_files = Vec::new();
        
        for file in all_files {
            let key = &file.filename;
            let has_permission = self.check_credential_permission(
                cred_id,
                did,
                key,
                "read",
            ).await?;
            
            if has_permission {
                accessible_files.push(file);
            }
        }
        
        Ok(accessible_files)
    }
    
    /// Check if a credential grants permission to a file
    async fn check_credential_permission(
        &self,
        credential_id: &str,
        did: &str,
        file_key: &str,
        permission: &str,
    ) -> Result<bool> {
        // Resolve the credential
        let credential = match self.credential_provider.resolve_credential(credential_id).await? {
            Some(cred) => cred,
            None => return Err(anyhow!("Credential not found: {}", credential_id)),
        };
        
        // Verify the credential
        let status = self.credential_provider.verify_credential(&credential).await?;
        if status != CredentialVerificationStatus::Verified {
            return Err(anyhow!("Credential verification failed: {:?}", status));
        }
        
        // Check if credential belongs to the DID
        if credential.credentialSubject.id != did {
            return Err(anyhow!("Credential does not belong to this DID"));
        }
        
        // Check if credential grants the requested permission
        self.check_credential_permissions(&credential, file_key, permission).await
    }
    
    /// Helper method to authenticate DID and get member ID
    async fn authenticate_and_get_member_id(
        &mut self,
        did: &str,
        challenge: &[u8],
        signature: &[u8],
    ) -> Result<String> {
        // Get a reference to the identity provider
        let identity_provider = self.identity_storage.get_identity_provider();
        
        // Authenticate the DID
        let status = self.identity_storage.authenticate_did(did, challenge, signature).await?;
        if status != DidVerificationStatus::Verified {
            return Err(anyhow!("DID authentication failed: {:?}", status));
        }
        
        // Get member ID
        identity_provider.did_to_member_id(did)
    }
    
    /// Create an access rule with DID authentication
    pub async fn create_access_rule(
        &mut self,
        did: &str,
        challenge: &[u8],
        signature: &[u8],
        rule: CredentialAccessRule,
    ) -> Result<()> {
        // Authenticate the DID
        let member_id = self.authenticate_and_get_member_id(did, challenge, signature).await?;
        
        // Add the rule
        self.add_access_rule(rule);
        
        Ok(())
    }
    
    /// Get the reference to the identity storage service
    pub fn get_identity_storage(&self) -> &IdentityStorageService<I> {
        &self.identity_storage
    }
    
    /// Get a mutable reference to the identity storage service
    pub fn get_identity_storage_mut(&mut self) -> &mut IdentityStorageService<I> {
        &mut self.identity_storage
    }
    
    /// Get the reference to the credential provider
    pub fn get_credential_provider(&self) -> &C {
        &self.credential_provider
    }
    
    /// Get a mutable reference to the credential provider
    pub fn get_credential_provider_mut(&mut self) -> &mut C {
        &mut self.credential_provider
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity_storage::{MockIdentityProvider, DidDocument, VerificationMethod, ServiceEndpoint};
    use tempfile::tempdir;
    
    // Helper function to create a test credential
    fn create_test_credential(
        id: &str,
        subject_id: &str,
        credential_type: &str,
        attributes: HashMap<String, serde_json::Value>,
    ) -> VerifiableCredential {
        let mut claims = serde_json::Map::new();
        for (key, value) in attributes {
            claims.insert(key, value);
        }
        
        VerifiableCredential {
            context: vec![
                "https://www.w3.org/2018/credentials/v1".to_string(),
                "https://www.w3.org/2018/credentials/examples/v1".to_string(),
            ],
            id: id.to_string(),
            credential_type: vec![
                "VerifiableCredential".to_string(),
                credential_type.to_string(),
            ],
            issuer: "did:icn:test:issuer".to_string(),
            issuanceDate: "2023-01-01T00:00:00Z".to_string(),
            expirationDate: Some("2033-01-01T00:00:00Z".to_string()),
            credentialSubject: CredentialSubject {
                id: subject_id.to_string(),
                claims,
            },
            proof: CredentialProof {
                proof_type: "Ed25519Signature2020".to_string(),
                created: "2023-01-01T00:00:00Z".to_string(),
                verificationMethod: "did:icn:test:issuer#key-1".to_string(),
                proofPurpose: "assertionMethod".to_string(),
                jws: Some("test_signature".to_string()),
                proof_value: None,
            },
            revocation: None,
        }
    }
    
    #[tokio::test]
    async fn test_credential_based_access() -> Result<()> {
        // Create a test DID
        let did = "did:icn:test:alice";
        let document = DidDocument {
            id: did.to_string(),
            controller: None,
            verification_method: vec![
                VerificationMethod {
                    id: format!("{}#keys-1", did),
                    type_: "Ed25519VerificationKey2020".to_string(),
                    controller: did.to_string(),
                    public_key_jwk: None,
                    public_key_multibase: Some("z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK".to_string()),
                }
            ],
            authentication: vec![format!("{}#keys-1", did)],
            service: vec![],
        };
        
        // Create a mock identity provider
        let mut identity_provider = MockIdentityProvider::new();
        identity_provider.add_did_document(did.to_string(), document);
        
        // Create test credentials
        let mut attributes = HashMap::new();
        attributes.insert("role".to_string(), serde_json::Value::String("admin".to_string()));
        let admin_credential = create_test_credential(
            "credential:1",
            did,
            "AdminCredential",
            attributes,
        );
        
        let mut attributes = HashMap::new();
        attributes.insert("department".to_string(), serde_json::Value::String("finance".to_string()));
        let finance_credential = create_test_credential(
            "credential:2",
            did,
            "DepartmentCredential",
            attributes,
        );
        
        // Create a mock credential provider
        let mut credential_provider = MockCredentialProvider::new();
        credential_provider.add_credential(admin_credential);
        credential_provider.add_credential(finance_credential);
        
        // Create a temporary directory for the test
        let temp_dir = tempdir()?;
        
        // Create a credential storage service
        let mut service = CredentialStorageService::new(
            "test",
            temp_dir.path(),
            identity_provider,
            credential_provider,
            3600, // 1 hour cache TTL
        ).await?;
        
        // Add some access rules
        service.add_access_rule(CredentialAccessRule {
            pattern: "admin_*".to_string(),
            credential_types: vec!["AdminCredential".to_string()],
            required_attributes: {
                let mut attrs = HashMap::new();
                attrs.insert("role".to_string(), serde_json::Value::String("admin".to_string()));
                attrs
            },
            permissions: vec!["read".to_string(), "write".to_string()],
        });
        
        service.add_access_rule(CredentialAccessRule {
            pattern: "finance_*".to_string(),
            credential_types: vec!["DepartmentCredential".to_string()],
            required_attributes: {
                let mut attrs = HashMap::new();
                attrs.insert("department".to_string(), serde_json::Value::String("finance".to_string()));
                attrs
            },
            permissions: vec!["read".to_string(), "write".to_string()],
        });
        
        // Test permission checks
        let challenge = b"test challenge";
        let signature = b"test signature";
        
        // Admin credential should grant access to admin files
        let has_admin_access = service.check_credential_permission(
            "credential:1", 
            did, 
            "admin_file.txt", 
            "read"
        ).await?;
        assert!(has_admin_access, "Admin credential should grant read access to admin files");
        
        // Finance credential should grant access to finance files
        let has_finance_access = service.check_credential_permission(
            "credential:2", 
            did, 
            "finance_report.txt", 
            "write"
        ).await?;
        assert!(has_finance_access, "Finance credential should grant write access to finance files");
        
        // Admin credential should not grant access to finance files
        let has_cross_access = service.check_credential_permission(
            "credential:1", 
            did, 
            "finance_report.txt", 
            "read"
        ).await?;
        assert!(!has_cross_access, "Admin credential should not grant access to finance files");
        
        Ok(())
    }
} 