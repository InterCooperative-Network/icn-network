//! Identity management for the InterCooperative Network

mod did;
mod credentials;
mod storage;
mod mock;
mod attestation;
mod reputation;
pub mod zkp;

// Re-export everything for backward compatibility
pub use did::*;
pub use credentials::*;
pub use storage::*;
pub use mock::*;
pub use attestation::*;
pub use reputation::*;

// Re-export lib.rs contents
pub use crate::identity::lib::*;

// Create a module from the former lib.rs 
mod lib {
    pub use super::*;
} 