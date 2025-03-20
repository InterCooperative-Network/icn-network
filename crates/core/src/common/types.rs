//! Common types shared across the ICN system

use std::collections::HashMap;
use std::sync::Arc;
use serde::{Serialize, Deserialize};

/// Decentralized Identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DID(String);

impl DID {
    /// Create a new DID
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the DID as a string
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for DID {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for DID {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Flexible Value type for VM operations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Value {
    /// Null value
    Null,
    /// Boolean value
    Bool(bool),
    /// Integer value
    Int(i64),
    /// Floating point value
    Float(f64),
    /// String value
    String(String),
    /// Binary data
    Bytes(Vec<u8>),
    /// Array of values
    Array(Vec<Value>),
    /// Map of string keys to values
    Object(HashMap<String, Value>),
    /// Entity ID reference
    EntityRef(String),
    /// DID reference
    DIDRef(DID),
}

/// Entity identifier
pub type EntityId = String;

/// Timestamp in seconds since epoch
pub type Timestamp = u64;

/// Operation context for VM execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationContext {
    /// The identity that called the operation
    pub caller: DID,
    /// Current timestamp
    pub timestamp: u64,
    /// Additional metadata for the operation
    pub metadata: HashMap<String, Value>,
} 