//! Transaction management for the mutual credit system.

use crate::types::{Amount, DID, Timestamp};
use icn_crypto::Signature;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::fmt;
use uuid::Uuid;

/// A unique identifier for a transaction
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TransactionId(Uuid);

impl TransactionId {
    /// Create a new random transaction ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create a transaction ID from an existing UUID
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Get the underlying UUID
    pub fn uuid(&self) -> &Uuid {
        &self.0
    }
}

impl fmt::Display for TransactionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Status of a transaction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionStatus {
    /// Transaction is pending
    Pending,
    /// Transaction is completed
    Completed,
    /// Transaction is rejected
    Rejected,
    /// Transaction is cancelled
    Cancelled,
}

/// Type of transaction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionType {
    /// Direct transfer between two accounts
    DirectTransfer,
    /// Transfer through a path of credit lines
    PathTransfer,
    /// Credit line adjustment
    CreditLineAdjustment,
    /// System operation
    SystemOperation,
}

/// A transaction in the mutual credit system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    /// Unique identifier for the transaction
    pub id: String,
    /// Sender account
    pub from: DID,
    /// Receiver account
    pub to: DID,
    /// Amount of the transaction
    pub amount: Amount,
    /// Description of the transaction
    pub description: Option<String>,
    /// Type of transaction
    pub transaction_type: TransactionType,
    /// Status of the transaction
    pub status: TransactionStatus,
    /// When the transaction was created
    pub created_at: Timestamp,
    /// When the transaction was last updated
    pub updated_at: Timestamp,
    /// Path of the transaction (for path transfers)
    pub path: Option<Vec<DID>>,
    /// Metadata for the transaction
    pub metadata: HashMap<String, JsonValue>,
}

impl Transaction {
    /// Create a new transaction
    pub fn new(
        id: String,
        from: DID,
        to: DID,
        amount: Amount,
        transaction_type: TransactionType,
        description: Option<String>,
    ) -> Self {
        let now = chrono::Utc::now();
        Self {
            id,
            from,
            to,
            amount,
            description,
            transaction_type,
            status: TransactionStatus::Pending,
            created_at: now,
            updated_at: now,
            path: None,
            metadata: HashMap::new(),
        }
    }

    /// Set the path for a path transfer
    pub fn with_path(mut self, path: Vec<DID>) -> Self {
        self.path = Some(path);
        self
    }

    /// Add metadata to the transaction
    pub fn add_metadata(&mut self, key: String, value: JsonValue) {
        self.metadata.insert(key, value);
        self.updated_at = chrono::Utc::now();
    }

    /// Complete the transaction
    pub fn complete(&mut self) {
        self.status = TransactionStatus::Completed;
        self.updated_at = chrono::Utc::now();
    }

    /// Reject the transaction
    pub fn reject(&mut self) {
        self.status = TransactionStatus::Rejected;
        self.updated_at = chrono::Utc::now();
    }

    /// Cancel the transaction
    pub fn cancel(&mut self) {
        self.status = TransactionStatus::Cancelled;
        self.updated_at = chrono::Utc::now();
    }

    /// Check if the transaction is pending
    pub fn is_pending(&self) -> bool {
        self.status == TransactionStatus::Pending
    }

    /// Check if the transaction is completed
    pub fn is_completed(&self) -> bool {
        self.status == TransactionStatus::Completed
    }

    /// Check if the transaction is rejected
    pub fn is_rejected(&self) -> bool {
        self.status == TransactionStatus::Rejected
    }

    /// Check if the transaction is cancelled
    pub fn is_cancelled(&self) -> bool {
        self.status == TransactionStatus::Cancelled
    }
}

/// Additional metadata for a transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionMetadata {
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Optional location data
    pub location: Option<GeoLocation>,
    /// Reference to external systems
    pub reference: Option<String>,
    /// Level of privacy for this transaction
    pub privacy_level: PrivacyLevel,
    /// Identifiers for any parent transactions (for path-based transactions)
    pub parent_transactions: Vec<TransactionId>,
    /// Custom fields for extensibility
    pub custom_fields: serde_json::Value,
}

impl TransactionMetadata {
    /// Create a new transaction metadata with default values
    pub fn new() -> Self {
        Self {
            tags: Vec::new(),
            location: None,
            reference: None,
            privacy_level: PrivacyLevel::ParticipantsOnly,
            parent_transactions: Vec::new(),
            custom_fields: serde_json::Value::Null,
        }
    }

    /// Create new transaction metadata with the specified privacy level
    pub fn with_privacy(privacy_level: PrivacyLevel) -> Self {
        Self {
            tags: Vec::new(),
            location: None,
            reference: None,
            privacy_level,
            parent_transactions: Vec::new(),
            custom_fields: serde_json::Value::Null,
        }
    }

    /// Add a tag to the transaction metadata
    pub fn add_tag(&mut self, tag: impl Into<String>) {
        self.tags.push(tag.into());
    }

    /// Add multiple tags to the transaction metadata
    pub fn add_tags(&mut self, tags: impl IntoIterator<Item = impl Into<String>>) {
        for tag in tags {
            self.tags.push(tag.into());
        }
    }

    /// Set the location for the transaction
    pub fn set_location(&mut self, location: GeoLocation) {
        self.location = Some(location);
    }

    /// Set an external reference for the transaction
    pub fn set_reference(&mut self, reference: impl Into<String>) {
        self.reference = Some(reference.into());
    }

    /// Add a parent transaction ID
    pub fn add_parent(&mut self, parent_id: TransactionId) {
        self.parent_transactions.push(parent_id);
    }

    /// Set custom fields for the transaction
    pub fn set_custom_fields(&mut self, fields: serde_json::Value) {
        self.custom_fields = fields;
    }
}

impl Default for TransactionMetadata {
    fn default() -> Self {
        Self::new()
    }
}

/// Geographic location data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoLocation {
    /// Latitude coordinate
    pub latitude: f64,
    /// Longitude coordinate
    pub longitude: f64,
    /// Optional location name
    pub name: Option<String>,
}

/// Privacy level for transactions
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrivacyLevel {
    /// Visible to all network participants
    Public,
    /// Visible only within the federation
    FederationOnly,
    /// Visible only to transaction participants
    ParticipantsOnly,
    /// Fully confidential with zero-knowledge proofs
    Confidential,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Amount;

    #[test]
    fn test_transaction_id() {
        let id1 = TransactionId::new();
        let id2 = TransactionId::new();
        
        assert_ne!(id1, id2);
        
        let uuid = Uuid::new_v4();
        let id3 = TransactionId::from_uuid(uuid);
        
        assert_eq!(id3.uuid(), &uuid);
        assert_eq!(id3.to_string(), uuid.to_string());
    }

    #[test]
    fn test_transaction_basics() {
        let from = DID::new("from");
        let to = DID::new("to");
        let amount = Amount::new(100);
        
        let mut tx = Transaction::new(
            "tx123".to_string(),
            from.clone(),
            to.clone(),
            amount.clone(),
            TransactionType::DirectTransfer,
            Some("Test transaction".to_string()),
        );
        
        assert_eq!(tx.from, from);
        assert_eq!(tx.to, to);
        assert_eq!(tx.amount, amount);
        assert_eq!(tx.transaction_type, TransactionType::DirectTransfer);
        assert_eq!(tx.status, TransactionStatus::Pending);
        assert!(tx.is_pending());
        
        // Complete the transaction
        tx.complete();
        assert_eq!(tx.status, TransactionStatus::Completed);
        assert!(tx.is_completed());
        
        // Add metadata
        tx.add_metadata("receipt_id".to_string(), JsonValue::String("R123".to_string()));
        assert!(tx.metadata.contains_key("receipt_id"));
    }

    #[test]
    fn test_transaction_with_path() {
        let from = DID::new("from");
        let to = DID::new("to");
        let intermediate1 = DID::new("intermediate1");
        let intermediate2 = DID::new("intermediate2");
        let amount = Amount::new(50);
        
        let path = vec![from.clone(), intermediate1, intermediate2, to.clone()];
        
        let tx = Transaction::new(
            "tx456".to_string(),
            from,
            to,
            amount,
            TransactionType::PathTransfer,
            None,
        )
        .with_path(path.clone());
        
        assert_eq!(tx.transaction_type, TransactionType::PathTransfer);
        assert!(tx.path.is_some());
        assert_eq!(tx.path.unwrap(), path);
    }

    #[test]
    fn test_transaction_status_changes() {
        let from = DID::new("from");
        let to = DID::new("to");
        let amount = Amount::new(75);
        
        let mut tx = Transaction::new(
            "tx789".to_string(),
            from,
            to,
            amount,
            TransactionType::DirectTransfer,
            None,
        );
        
        assert!(tx.is_pending());
        
        // Reject the transaction
        tx.reject();
        assert_eq!(tx.status, TransactionStatus::Rejected);
        assert!(tx.is_rejected());
        assert!(!tx.is_pending());
        
        // Cancel the transaction (though this wouldn't normally happen after rejection)
        tx.cancel();
        assert_eq!(tx.status, TransactionStatus::Cancelled);
        assert!(tx.is_cancelled());
    }
} 