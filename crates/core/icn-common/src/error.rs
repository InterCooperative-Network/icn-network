//! Error types for the ICN project

use thiserror::Error;
use std::result;

/// Common result type used throughout ICN
pub type Result<T> = result::Result<T, Error>;

/// Common error type for ICN components
#[derive(Error, Debug)]
pub enum Error {
    /// Input/output error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
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
    #[error("Not found: {0}")]
    NotFound(String),
    
    /// Already exists error
    #[error("Resource already exists: {0}")]
    AlreadyExists(String),
    
    /// Not implemented error
    #[error("Not implemented: {0}")]
    NotImplemented(String),
    
    /// Network error
    #[error("Network error: {0}")]
    Network(String),
    
    /// Security error
    #[error("Security error: {0}")]
    Security(String),
    
    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),
}

impl Error {
    /// Create a new validation error
    pub fn validation<S: Into<String>>(msg: S) -> Self {
        Error::Validation(msg.into())
    }
    
    /// Create a new configuration error
    pub fn configuration<S: Into<String>>(msg: S) -> Self {
        Error::Configuration(msg.into())
    }

    /// Create a new not found error
    pub fn not_found<S: Into<String>>(msg: S) -> Self {
        Error::NotFound(msg.into())
    }

    /// Create a new not implemented error
    pub fn not_implemented<S: Into<String>>(msg: S) -> Self {
        Error::NotImplemented(msg.into())
    }

    /// Create a new network error
    pub fn network<S: Into<String>>(msg: S) -> Self {
        Error::Network(msg.into())
    }

    /// Create a new security error
    pub fn security<S: Into<String>>(msg: S) -> Self {
        Error::Security(msg.into())
    }

    /// Create a new internal error
    pub fn internal<S: Into<String>>(msg: S) -> Self {
        Error::Internal(msg.into())
    }
}

/// Error type for component shutdown operations
#[derive(Debug, Error)]
pub enum ShutdownError {
    #[error("Component is still running")]
    StillRunning,
    
    #[error("Shutdown failed: {0}")]
    Failed(String),
}
