//! Core ICN module
//!
//! This module provides fundamental components for the InterCooperative Network,
//! including networking, storage, and cryptography.

pub mod storage;
pub mod networking;
pub mod crypto;
pub mod config;
pub mod utils;

// Re-export key components
pub use storage::{Storage, StorageResult, StorageError, FileStorage};

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