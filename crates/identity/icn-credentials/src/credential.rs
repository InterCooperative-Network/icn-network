//! Credential types for the ICN verifiable credentials system

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The subject of a credential
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CredentialSubject {
    /// Optional DID of the subject
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    
    /// Arbitrary properties for this credential subject
    #[serde(flatten)]
    pub properties: HashMap<String, serde_json::Value>,
}

impl CredentialSubject {
    /// Create a new credential subject
    pub fn new(id: Option<String>) -> Self {
        CredentialSubject {
            id,
            properties: HashMap::new(),
        }
    }
    
    /// Add a property to the credential subject
    pub fn add_property<T: Into<serde_json::Value>>(&mut self, name: &str, value: T) {
        self.properties.insert(name.to_string(), value.into());
    }
    
    /// Get a property of the credential subject
    pub fn get_property(&self, name: &str) -> Option<&serde_json::Value> {
        self.properties.get(name)
    }
}

/// Status information for a credential
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CredentialStatus {
    /// Identifier of this status
    pub id: String,
    
    /// Type of this status
    #[serde(rename = "type")]
    pub type_: String,
    
    /// Status-specific properties
    #[serde(flatten)]
    pub properties: HashMap<String, serde_json::Value>,
}

/// Main credential type, which is aliased to VerifiableCredential in the main module
/// This is here to satisfy the re-export in lib.rs
pub type Credential = super::VerifiableCredential;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_credential_subject() {
        let mut subject = CredentialSubject::new(Some("did:icn:test:123".to_string()));
        subject.add_property("name", "John Doe");
        subject.add_property("age", 30);
        subject.add_property("isCooperativeMember", true);
        
        assert_eq!(subject.id, Some("did:icn:test:123".to_string()));
        assert_eq!(subject.get_property("name").unwrap().as_str().unwrap(), "John Doe");
        assert_eq!(subject.get_property("age").unwrap().as_i64().unwrap(), 30);
        assert_eq!(subject.get_property("isCooperativeMember").unwrap().as_bool().unwrap(), true);
    }
    
    #[test]
    fn test_credential_status() {
        let mut properties = HashMap::new();
        properties.insert("revocationListIndex".to_string(), serde_json::json!(12));
        properties.insert("revocationListCredential".to_string(), 
                          serde_json::json!("https://icn.coop/credentials/status/list1"));
        
        let status = CredentialStatus {
            id: "https://icn.coop/credentials/status/123".to_string(),
            type_: "RevocationList2023".to_string(),
            properties,
        };
        
        assert_eq!(status.id, "https://icn.coop/credentials/status/123");
        assert_eq!(status.type_, "RevocationList2023");
        assert_eq!(status.properties.get("revocationListIndex").unwrap().as_i64().unwrap(), 12);
    }
} 