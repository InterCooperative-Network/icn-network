//! Error types for the mutual credit system.

use thiserror::Error;

/// Errors that can occur in the mutual credit system
#[derive(Error, Debug)]
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
    #[error("No path found: {0}")]
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
    
    /// Other error
    #[error("Other error: {0}")]
    Other(String),
}

impl From<CreditError> for icn_common::error::Error {
    fn from(err: CreditError) -> Self {
        match err {
            CreditError::AccountAlreadyExists(msg) => icn_common::error::Error::validation(msg),
            CreditError::AccountNotFound(msg) => icn_common::error::Error::not_found(msg),
            CreditError::InactiveAccount(msg) => icn_common::error::Error::validation(msg),
            CreditError::CreditLineAlreadyExists(msg) => icn_common::error::Error::validation(msg),
            CreditError::CreditLineNotFound(msg) => icn_common::error::Error::not_found(msg),
            CreditError::InactiveCredit(msg) => icn_common::error::Error::validation(msg),
            CreditError::CreditLimitExceeded(msg) => icn_common::error::Error::validation(msg),
            CreditError::InsufficientFunds(msg) => icn_common::error::Error::validation(msg),
            CreditError::InvalidTransaction(msg) => icn_common::error::Error::validation(msg),
            CreditError::NoPathFound(msg) => icn_common::error::Error::not_found(msg),
            CreditError::NotImplemented(msg) => icn_common::error::Error::internal(msg),
            CreditError::SerializationError(msg) => icn_common::error::Error::internal(msg),
            CreditError::DeserializationError(msg) => icn_common::error::Error::internal(msg),
            CreditError::StorageError(msg) => icn_common::error::Error::internal(msg),
            CreditError::Other(msg) => icn_common::error::Error::internal(msg),
        }
    }
}

impl From<icn_common::error::Error> for CreditError {
    fn from(err: icn_common::error::Error) -> Self {
        let msg = err.to_string();
        
        if msg.contains("not found") {
            CreditError::AccountNotFound(msg)
        } else if msg.contains("already exists") {
            CreditError::AccountAlreadyExists(msg)
        } else if msg.contains("inactive") {
            CreditError::InactiveAccount(msg)
        } else if msg.contains("credit limit") {
            CreditError::CreditLimitExceeded(msg)
        } else if msg.contains("insufficient funds") {
            CreditError::InsufficientFunds(msg)
        } else if msg.contains("invalid transaction") {
            CreditError::InvalidTransaction(msg)
        } else if msg.contains("serialization") {
            CreditError::SerializationError(msg)
        } else if msg.contains("deserialization") {
            CreditError::DeserializationError(msg)
        } else if msg.contains("storage") {
            CreditError::StorageError(msg)
        } else {
            CreditError::Other(msg)
        }
    }
} 