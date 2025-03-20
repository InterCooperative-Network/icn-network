use thiserror::Error;
use std::fmt;

/// Common error type that can be used across crates
#[derive(Error, Debug)]
pub enum CommonError {
    #[error("Storage error: {0}")]
    Storage(String),
    
    #[error("Identity error: {0}")]
    Identity(String),
    
    #[error("Network error: {0}")]
    Network(String),
    
    #[error("Governance error: {0}")]
    Governance(String),
    
    #[error("Economic error: {0}")]
    Economic(String),
    
    #[error("VM error: {0}")]
    VM(String),
    
    #[error("DSL error: {0}")]
    DSL(String),
    
    #[error("Validation error: {0}")]
    Validation(String),
    
    #[error("Not implemented: {0}")]
    NotImplemented(String),

    #[error("IO error: {0}")]
    IO(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Result type alias using CommonError
pub type Result<T> = std::result::Result<T, CommonError>; 