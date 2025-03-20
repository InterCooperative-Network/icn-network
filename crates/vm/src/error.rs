//! Error types for VM operations

use thiserror::Error;
use icn_common::error::{CommonError, Result as CommonResult};

/// Error types for VM operations
#[derive(Error, Debug)]
pub enum VMError {
    #[error("Execution error: {0}")]
    ExecutionError(String),
    
    #[error("State error: {0}")]
    StateError(String),
    
    #[error("Security error: {0}")]
    SecurityError(String),
    
    #[error("Storage error: {0}")]
    StorageError(String),
    
    #[error("Permission error: {0}")]
    PermissionError(String),
    
    #[error("Validation error: {0}")]
    ValidationError(String),
    
    #[error("Not found: {0}")]
    NotFound(String),
    
    #[error("Internal error: {0}")]
    InternalError(String),
}

// Convert from CommonError to VMError
impl From<CommonError> for VMError {
    fn from(err: CommonError) -> Self {
        match err {
            CommonError::Storage(msg) => VMError::StorageError(msg),
            CommonError::Validation(msg) => VMError::ValidationError(msg),
            CommonError::Governance(msg) => VMError::ExecutionError(format!("Governance error: {}", msg)),
            _ => VMError::InternalError(err.to_string()),
        }
    }
}

// Convert from VMError to CommonError
impl From<VMError> for CommonError {
    fn from(err: VMError) -> Self {
        CommonError::VM(err.to_string())
    }
}

/// Result type for VM operations
pub type Result<T> = std::result::Result<T, VMError>; 