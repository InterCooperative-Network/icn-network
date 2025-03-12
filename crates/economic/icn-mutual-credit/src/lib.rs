//! Mutual credit implementation for the Intercooperative Network.
//!
//! This crate provides the core functionality for a mutual credit system,
//! including account management, credit lines, transactions, and credit graph.

mod account;
mod credit_graph;
mod credit_line;
mod error;
mod transaction;
mod transaction_processor;
mod types;

pub use account::{Account, AccountStatus};
pub use credit_graph::{CreditGraph, CreditLineId, CreditLineStep};
pub use credit_line::{
    CollateralRequirement, CollateralType, CreditCondition, CreditLine, CreditTerms, ResourceCommitment,
};
pub use error::CreditError;
pub use transaction::{Transaction, TransactionStatus, TransactionType};
pub use transaction_processor::{TransactionProcessor, TransactionResult, CreditClearingParams};
pub use types::{Amount, DID, Timestamp};

/// Version of the mutual credit implementation
pub const VERSION: &str = env!("CARGO_PKG_VERSION"); 