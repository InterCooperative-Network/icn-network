//! Digital identity for ICN
//!
//! This module provides digital identity capabilities for nodes in the
//! InterCooperative Network, based on Ed25519 cryptography.

use std::fmt;
use std::path::Path;
use thiserror::Error;
use serde::{Serialize, Deserialize};
use tokio::fs;
use super::{KeyPairWrapper, Signature, Hash, sha256, CryptoResult, CryptoError};

/// An identity key pair for a node
pub struct IdentityKeyPair {
    /// The underlying Ed25519 key pair
    key_pair: KeyPairWrapper,
    /// The node ID derived from the public key
    node_id: NodeId,
}

impl IdentityKeyPair {
    /// Generate a new random identity key pair
    pub fn generate() -> CryptoResult<Self> {
        let key_pair = KeyPairWrapper::generate()?;
        let node_id = NodeId::from_public_key(key_pair.public_key_bytes());
        
        Ok(Self {
            key_pair,
            node_id,
        })
    }
    
    /// Create an IdentityKeyPair from PKCS#8 encoded bytes
    pub fn from_pkcs8(pkcs8_bytes: &[u8]) -> CryptoResult<Self> {
        let key_pair = KeyPairWrapper::from_pkcs8(pkcs8_bytes)?;
        let node_id = NodeId::from_public_key(key_pair.public_key_bytes());
        
        Ok(Self {
            key_pair,
            node_id,
        })
    }
    
    /// Load an identity key pair from a file
    pub async fn load_from_file<P: AsRef<Path>>(path: P) -> CryptoResult<Self> {
        let pkcs8_bytes = fs::read(path).await
            .map_err(|e| CryptoError::IoError(e))?;
        
        Self::from_pkcs8(&pkcs8_bytes)
    }
    
    /// Save an identity key pair to a file
    pub async fn save_to_file<P: AsRef<Path>>(&self, path: P) -> CryptoResult<()> {
        // In a real implementation, we would need to securely handle the private key
        // For this example, we're just saving the PKCS#8 bytes
        // You'd want to encrypt this with a password in a real system
        
        // This is a placeholder - in reality you'd need to extract the PKCS#8 bytes
        let pkcs8_bytes = vec![]; // placeholder
        
        fs::write(path, &pkcs8_bytes).await
            .map_err(|e| CryptoError::IoError(e))?;
        
        Ok(())
    }
    
    /// Get the node ID for this identity
    pub fn node_id(&self) -> &NodeId {
        &self.node_id
    }
    
    /// Get the public key bytes
    pub fn public_key_bytes(&self) -> &[u8] {
        self.key_pair.public_key_bytes()
    }
    
    /// Sign a message
    pub fn sign(&self, message: &[u8]) -> Signature {
        self.key_pair.sign(message)
    }
    
    /// Create a signed message with this identity
    pub fn create_signed_message<T: Serialize>(&self, content: &T) -> CryptoResult<SignedMessage<T>> {
        let content_bytes = serde_json::to_vec(content)
            .map_err(|e| CryptoError::SerializationError(format!("Failed to serialize content: {}", e)))?;
        
        let signature = self.sign(&content_bytes);
        
        Ok(SignedMessage {
            content: content.clone(),
            signature,
            signer: self.node_id.clone(),
        })
    }
}

/// A node ID derived from a public key
#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(String);

impl NodeId {
    /// Create a NodeId from a public key
    pub fn from_public_key(public_key: &[u8]) -> Self {
        let hash = sha256(public_key);
        // Use the first 16 bytes of the hash as a base58 string
        let id = bs58::encode(&hash.as_bytes()[0..16]).into_string();
        Self(id)
    }
    
    /// Create a NodeId from a string
    pub fn from_string(id: impl Into<String>) -> Self {
        Self(id.into())
    }
    
    /// Get the string representation
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NodeId({})", self.0)
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A message signed by an identity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedMessage<T> {
    /// The content of the message
    pub content: T,
    /// The signature of the content
    pub signature: Signature,
    /// The signer's node ID
    pub signer: NodeId,
}

impl<T: Serialize> SignedMessage<T> {
    /// Verify the signature of this message
    pub fn verify(&self, public_key: &[u8]) -> CryptoResult<bool> {
        let content_bytes = serde_json::to_vec(&self.content)
            .map_err(|e| CryptoError::SerializationError(format!("Failed to serialize content: {}", e)))?;
        
        // First, verify that the node ID matches the public key
        let expected_node_id = NodeId::from_public_key(public_key);
        if expected_node_id != self.signer {
            return Ok(false);
        }
        
        // Then, verify the signature
        match super::verify_signature(public_key, &content_bytes, &self.signature) {
            Ok(()) => Ok(true),
            Err(_) => Ok(false),
        }
    }
} 