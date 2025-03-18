/// ICN Command Line Interface Library
///
/// This crate provides the command-line interface for interacting with the ICN Network,
/// as well as shared functionality like the DSL system.

// Re-export the DSL module for use by other crates
pub mod dsl;

// Re-export other modules as needed
pub mod storage;
pub mod compute;
pub mod governance;
pub mod distributed;
pub mod governance_storage;
pub mod identity_storage;
pub mod credential_storage;
pub mod networking;

// Version and build information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION"); 