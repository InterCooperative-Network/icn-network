//! Intercooperative Network (ICN) - A decentralized infrastructure for cooperative economies
//!
//! This crate provides the core functionality for the Intercooperative Network,
//! a decentralized infrastructure designed to support cooperative economic activities.

pub use icn_common as common;
pub use icn_crypto as crypto;
pub use icn_mutual_credit as economic;

/// Module version information
pub mod version {
    /// Version of the ICN implementation
    pub const VERSION: &str = env!("CARGO_PKG_VERSION");
    
    /// Major version number
    pub const MAJOR: u32 = 0;
    
    /// Minor version number
    pub const MINOR: u32 = 1;
    
    /// Patch version number
    pub const PATCH: u32 = 0;
}
