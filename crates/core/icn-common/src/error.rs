//! Error types for the Intercooperative Network

use thiserror::Error;
use std::result;

/// Common result type used throughout ICN
pub type Result<T> = result::Result<T, Error>;

/// Common error type for the Intercooperative Network
#[derive(Error, Debug)]
pub enum Error {
    /// Validation error
    #[error("Validation error: {0}")]
    Validation(String),

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),

    /// Not found error
    #[error("Not found: {0}")]
    NotFound(String),

    /// Unauthorized error
    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Deserialization error
    #[error("Deserialization error: {0}")]
    Deserialization(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Other error
    #[error("{0}")]
    Other(String),
}

impl Error {
    /// Create a new validation error
    pub fn validation<S: Into<String>>(msg: S) -> Self {
        Error::Validation(msg.into())
    }

    /// Create a new internal error
    pub fn internal<S: Into<String>>(msg: S) -> Self {
        Error::Internal(msg.into())
    }

    /// Create a new not found error
    pub fn not_found<S: Into<String>>(msg: S) -> Self {
        Error::NotFound(msg.into())
    }

    /// Create a new unauthorized error
    pub fn unauthorized<S: Into<String>>(msg: S) -> Self {
        Error::Unauthorized(msg.into())
    }

    /// Create a new serialization error
    pub fn serialization<S: Into<String>>(msg: S) -> Self {
        Error::Serialization(msg.into())
    }

    /// Create a new deserialization error
    pub fn deserialization<S: Into<String>>(msg: S) -> Self {
        Error::Deserialization(msg.into())
    }

    /// Create a new other error
    pub fn other<S: Into<String>>(msg: S) -> Self {
        Error::Other(msg.into())
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
