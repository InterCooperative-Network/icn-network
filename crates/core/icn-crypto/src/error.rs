//! Error types for cryptographic operations

use thiserror::Error;
use icn_common::Error as CommonError;

/// Error type for cryptographic operations
#[derive(Error, Debug)]
pub enum CryptoError {
    /// Invalid key
    #[error("Invalid key: {0}")]
    InvalidKey(String),

    /// Invalid signature
    #[error("Invalid signature: {0}")]
    InvalidSignature(String),

    /// Verification failed
    #[error("Verification failed: {0}")]
    VerificationFailed(String),

    /// Signing failed
    #[error("Signing failed: {0}")]
    SigningFailed(String),

    /// Encryption failed
    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),

    /// Decryption failed
    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),

    /// Unsupported algorithm
    #[error("Unsupported algorithm: {0}")]
    UnsupportedAlgorithm(String),

    /// Other error
    #[error("{0}")]
    Other(String),
}

impl From<CryptoError> for CommonError {
    fn from(err: CryptoError) -> Self {
        match err {
            CryptoError::InvalidKey(msg) | 
            CryptoError::InvalidSignature(msg) | 
            CryptoError::VerificationFailed(msg) => CommonError::validation(msg),
            
            CryptoError::SigningFailed(msg) | 
            CryptoError::EncryptionFailed(msg) | 
            CryptoError::DecryptionFailed(msg) | 
            CryptoError::UnsupportedAlgorithm(msg) => CommonError::internal(msg),
            
            CryptoError::Other(msg) => CommonError::other(msg),
        }
    }
}

/// Result type for cryptographic operations
pub type Result<T> = std::result::Result<T, CryptoError>; 