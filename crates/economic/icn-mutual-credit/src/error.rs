//! Error types for the mutual credit system.

use std::fmt;
use thiserror::Error;
use crate::confidential::ConfidentialError;

/// Errors that can occur in the mutual credit system
#[derive(Debug, Error)]
pub enum CreditError {
    /// Account already exists
    #[error("Account already exists: {0}")]
    AccountAlreadyExists(String),
    
    /// Account not found
    #[error("Account not found: {0}")]
    AccountNotFound(String),
    
    /// Account is inactive
    #[error("Account is inactive: {0}")]
    InactiveAccount(String),
    
    /// Credit line already exists
    #[error("Credit line already exists: {0}")]
    CreditLineAlreadyExists(String),
    
    /// Credit line not found
    #[error("Credit line not found: {0}")]
    CreditLineNotFound(String),
    
    /// Credit line is inactive
    #[error("Credit line is inactive: {0}")]
    InactiveCredit(String),
    
    /// Credit limit exceeded
    #[error("Credit limit exceeded: {0}")]
    CreditLimitExceeded(String),
    
    /// Insufficient funds
    #[error("Insufficient funds: {0}")]
    InsufficientFunds(String),
    
    /// Invalid transaction
    #[error("Invalid transaction: {0}")]
    InvalidTransaction(String),
    
    /// No path found
    #[error("No path found for transaction: {0}")]
    NoPathFound(String),
    
    /// Feature not implemented
    #[error("Not implemented: {0}")]
    NotImplemented(String),
    
    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    /// Deserialization error
    #[error("Deserialization error: {0}")]
    DeserializationError(String),
    
    /// Storage error
    #[error("Storage error: {0}")]
    StorageError(String),
    
    /// Not found error
    #[error("Not found: {0}")]
    NotFound(String),
    
    /// Validation error
    #[error("Validation error: {0}")]
    Validation(String),
    
    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),
    
    /// Other error
    #[error("Other error: {0}")]
    Other(String),
}

/// Result type for credit operations
pub type Result<T> = std::result::Result<T, CreditError>;

impl From<serde_json::Error> for CreditError {
    fn from(err: serde_json::Error) -> Self {
        if err.is_data() {
            Self::DeserializationError(err.to_string())
        } else {
            Self::SerializationError(err.to_string())
        }
    }
}

impl From<ConfidentialError> for CreditError {
    fn from(error: ConfidentialError) -> Self {
        match error {
            ConfidentialError::CryptoError(msg) => {
                CreditError::Other(format!("Crypto error: {}", msg))
            }
            ConfidentialError::InvalidCommitment(msg) => {
                CreditError::InvalidTransaction(format!("Invalid commitment: {}", msg))
            }
            ConfidentialError::ValidationError(msg) => {
                CreditError::InvalidTransaction(format!("Validation error: {}", msg))
            }
            ConfidentialError::ProofError(msg) => {
                CreditError::InvalidTransaction(format!("Proof error: {}", msg))
            }
            ConfidentialError::AmountRangeError(msg) => {
                CreditError::InvalidTransaction(format!("Amount range error: {}", msg))
            }
            ConfidentialError::BlindingError(msg) => {
                CreditError::Other(format!("Blinding error: {}", msg))
            }
        }
    }
} 