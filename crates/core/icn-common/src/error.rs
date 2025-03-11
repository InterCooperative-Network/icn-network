//! Error types for the ICN project

use thiserror::Error;

/// Common result type used throughout ICN
pub type Result<T> = std::result::Result<T, Error>;

/// Common error type for ICN components
#[derive(Error, Debug)]
pub enum Error {
    /// Input/output error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    
    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),
    
    /// Validation error
    #[error("Validation error: {0}")]
    Validation(String),
    
    /// Configuration error
    #[error("Configuration error: {0}")]
    Configuration(String),
    
    /// Authentication error
    #[error("Authentication error: {0}")]
    Authentication(String),
    
    /// Authorization error
    #[error("Authorization error: {0}")]
    Authorization(String),
    
    /// Not found error
    #[error("Resource not found: {0}")]
    NotFound(String),
    
    /// Already exists error
    #[error("Resource already exists: {0}")]
    AlreadyExists(String),
    
    /// Other error
    #[error("{0}")]
    Other(String),
}

impl Error {
    /// Create a new serialization error
    pub fn serialization<S: Into<String>>(msg: S) -> Self {
        Error::Serialization(msg.into())
    }
    
    /// Create a new validation error
    pub fn validation<S: Into<String>>(msg: S) -> Self {
        Error::Validation(msg.into())
    }
    
    /// Create a new configuration error
    pub fn configuration<S: Into<String>>(msg: S) -> Self {
        Error::Configuration(msg.into())
    }
    
    /// Create a new other error
    pub fn other<S: Into<String>>(msg: S) -> Self {
        Error::Other(msg.into())
    }
}
