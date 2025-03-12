use thiserror::Error;
use std::io;

/// Networking result type
pub type Result<T> = std::result::Result<T, NetworkError>;

/// Networking errors
#[derive(Error, Debug)]
pub enum NetworkError {
    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    /// TLS error
    #[error("TLS error: {0}")]
    Tls(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Connection error
    #[error("Connection error: {0}")]
    Connection(String),

    /// Peer not found
    #[error("Peer not found: {0}")]
    PeerNotFound(String),

    /// Validation error
    #[error("Validation error: {0}")]
    Validation(String),

    /// Lock error
    #[error("Lock error")]
    LockError,

    /// Channel closed
    #[error("Channel closed")]
    ChannelClosed,

    /// Protocol error
    #[error("Protocol error: {0}")]
    Protocol(String),

    /// Authentication error
    #[error("Authentication error: {0}")]
    Authentication(String),

    /// Authorization error
    #[error("Authorization error: {0}")]
    Authorization(String),

    /// Other error
    #[error("Other error: {0}")]
    Other(String),
}

impl From<&str> for NetworkError {
    fn from(s: &str) -> Self {
        NetworkError::Other(s.to_string())
    }
}

impl From<String> for NetworkError {
    fn from(s: String) -> Self {
        NetworkError::Other(s)
    }
}

impl From<std::sync::mpsc::RecvError> for NetworkError {
    fn from(_: std::sync::mpsc::RecvError) -> Self {
        NetworkError::ChannelClosed
    }
}

impl From<tokio::sync::mpsc::error::SendError<T>> for NetworkError 
where
    T: std::fmt::Debug,
{
    fn from(err: tokio::sync::mpsc::error::SendError<T>) -> Self {
        NetworkError::Other(format!("Channel send error: {:?}", err))
    }
}

impl From<NetworkError> for io::Error {
    fn from(err: NetworkError) -> Self {
        match err {
            NetworkError::Io(io_error) => io_error,
            _ => io::Error::new(io::ErrorKind::Other, err.to_string()),
        }
    }
}