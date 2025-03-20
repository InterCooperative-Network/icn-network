use std::fmt;
use std::error::Error;

/// Result type for networking operations
pub type Result<T> = std::result::Result<T, NetworkError>;

/// Network-related errors
#[derive(Debug)]
pub enum NetworkError {
    /// Connection error
    ConnectionError(String),
    
    /// Protocol error
    ProtocolError(String),
    
    /// Authentication error
    AuthenticationError(String),
    
    /// Authorization error
    AuthorizationError(String),
    
    /// Timeout error
    TimeoutError(String),
    
    /// Address error
    AddressError(String),
    
    /// Routing error
    RoutingError(String),
    
    /// DHT error
    DhtError(String),
    
    /// Tunnel error
    TunnelError(String),
    
    /// IO error
    IoError(String),
    
    /// Serialization error
    SerializationError(String),
    
    /// Configuration error
    ConfigurationError(String),
    
    /// Other error
    Other(String),
}

impl fmt::Display for NetworkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NetworkError::ConnectionError(msg) => write!(f, "Connection error: {}", msg),
            NetworkError::ProtocolError(msg) => write!(f, "Protocol error: {}", msg),
            NetworkError::AuthenticationError(msg) => write!(f, "Authentication error: {}", msg),
            NetworkError::AuthorizationError(msg) => write!(f, "Authorization error: {}", msg),
            NetworkError::TimeoutError(msg) => write!(f, "Timeout error: {}", msg),
            NetworkError::AddressError(msg) => write!(f, "Address error: {}", msg),
            NetworkError::RoutingError(msg) => write!(f, "Routing error: {}", msg),
            NetworkError::DhtError(msg) => write!(f, "DHT error: {}", msg),
            NetworkError::TunnelError(msg) => write!(f, "Tunnel error: {}", msg),
            NetworkError::IoError(msg) => write!(f, "IO error: {}", msg),
            NetworkError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            NetworkError::ConfigurationError(msg) => write!(f, "Configuration error: {}", msg),
            NetworkError::Other(msg) => write!(f, "Other error: {}", msg),
        }
    }
}

impl Error for NetworkError {}

impl From<std::io::Error> for NetworkError {
    fn from(err: std::io::Error) -> Self {
        NetworkError::IoError(err.to_string())
    }
}

impl From<serde_json::Error> for NetworkError {
    fn from(err: serde_json::Error) -> Self {
        NetworkError::SerializationError(err.to_string())
    }
}

impl From<tokio::time::error::Elapsed> for NetworkError {
    fn from(_: tokio::time::error::Elapsed) -> Self {
        NetworkError::TimeoutError("Operation timed out".to_string())
    }
} 