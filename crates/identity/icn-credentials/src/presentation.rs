//! Presentation types for the ICN verifiable credentials system
//!
//! This module provides the structures and functions for creating and managing
//! verifiable presentations of credentials.

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use icn_common::Result;

/// Options for creating a verifiable presentation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PresentationOptions {
    /// Challenge for proving control of the presentation
    pub challenge: Option<String>,
    
    /// Domain for restricting the presentation
    pub domain: Option<String>,
    
    /// Specific verification methods to use
    pub verification_method: Option<String>,
    
    /// When the presentation expires
    pub expires: Option<DateTime<Utc>>,
}

impl Default for PresentationOptions {
    fn default() -> Self {
        PresentationOptions {
            challenge: None,
            domain: None,
            verification_method: None,
            expires: None,
        }
    }
}

/// Main presentation type, which is aliased to VerifiablePresentation in the main module
/// This is here to satisfy the re-export in lib.rs
pub type Presentation = super::VerifiablePresentation;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Credential;
    use crate::CredentialSubject;
    
    #[test]
    fn test_presentation_options() {
        let options = PresentationOptions {
            challenge: Some("1234567890".to_string()),
            domain: Some("icn.coop".to_string()),
            verification_method: Some("did:icn:test:123#keys-1".to_string()),
            expires: Some(Utc::now()),
        };
        
        assert!(options.challenge.is_some());
        assert!(options.domain.is_some());
        assert!(options.verification_method.is_some());
        assert!(options.expires.is_some());
    }
    
    #[test]
    fn test_default_options() {
        let options = PresentationOptions::default();
        
        assert!(options.challenge.is_none());
        assert!(options.domain.is_none());
        assert!(options.verification_method.is_none());
        assert!(options.expires.is_none());
    }
} 