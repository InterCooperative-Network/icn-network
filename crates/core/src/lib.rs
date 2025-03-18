/// Core functionality for the ICN Network
///
/// This crate provides core functionality for the ICN Network,
/// including common traits, data structures, and utilities.

/// Common Error type for ICN crates
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Error from I/O operation
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    
    /// Error from serialization or deserialization
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    /// Error from parsing
    #[error("Parse error: {0}")]
    Parse(String),
    
    /// Error from network operation
    #[error("Network error: {0}")]
    Network(String),
    
    /// Error from storage operation
    #[error("Storage error: {0}")]
    Storage(String),
    
    /// Error from economic operation
    #[error("Economic error: {0}")]
    Economic(String),
    
    /// Error from governance operation
    #[error("Governance error: {0}")]
    Governance(String),
    
    /// Error from DSL operation
    #[error("DSL error: {0}")]
    Dsl(String),
}

/// Common Result type for ICN crates
pub type Result<T> = std::result::Result<T, Error>;

pub mod storage;
pub mod networking;
pub mod crypto;
pub mod config;
pub mod utils;

// Re-export key components
pub use storage::Storage;

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
/// Package description
pub const DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");

/// Initialize tracing for ICN
pub fn init_tracing() {
    use tracing_subscriber::FmtSubscriber;
    
    // Initialize the default tracing subscriber
    let subscriber = FmtSubscriber::builder()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .finish();
    
    // Set the subscriber as the global default
    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set global tracing subscriber");
}

// Core functionality for the ICN system

/// Common utilities
pub mod common {
    /// Common types for ICN
    pub mod types {
        /// A simple type alias for a hash
        pub type Hash = String;
    }
}

/// Initialize ICN core system
pub async fn init() -> Result<()> {
    init_tracing();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Storage;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::RwLock;
    
    // Test implementations and code
} 