use std::collections::{HashMap, HashSet};
use serde::{Serialize, Deserialize};

/// Account balance structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountBalance {
    /// Account ID
    pub account_id: String,
    /// Current balance
    pub balance: f64,
    /// Available credit (includes credit limit)
    pub available_credit: f64,
    /// Credit limit (negative)
    pub credit_limit: f64,
}

/// Transaction structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    /// Unique transaction ID
    pub id: String,
    /// Transaction type
    pub transaction_type: TransactionType,
    /// Source account ID
    pub from_account: String,
    /// Destination account ID (if applicable)
    pub to_account: Option<String>,
    /// Transaction amount
    pub amount: f64,
    /// Currency code
    pub currency: String,
    /// Transaction description
    pub description: String,
    /// Transaction status
    pub status: TransactionStatus,
    /// Created timestamp
    pub created_at: u64,
    /// Updated timestamp
    pub updated_at: u64,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
    /// Related transaction references
    pub references: Vec<String>,
}

/// Transaction type enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TransactionType {
    /// Simple transfer between accounts
    Transfer,
    /// Issuance of new value
    Issuance,
    /// Clearing of mutual credit
    Clearing,
    /// Fee collection
    Fee,
}

/// Transaction status enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TransactionStatus {
    /// Pending approval
    Pending,
    /// Approved but not yet confirmed
    Approved,
    /// Confirmed and completed
    Confirmed,
    /// Rejected
    Rejected,
    /// Cancelled
    Cancelled,
}

/// Mutual credit system
pub struct MutualCreditSystem {
    /// System configuration
    config: MutualCreditConfig,
    /// Account balances
    balances: HashMap<String, AccountBalance>,
    /// Transactions
    transactions: HashMap<String, Transaction>,
}

/// Mutual credit system configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutualCreditConfig {
    /// Default credit limit
    pub default_credit_limit: f64,
    /// Max negative balance
    pub max_negative_balance: f64,
    /// Transaction fee percentage
    pub fee_percentage: f64,
    /// Clearing interval
    pub clearing_interval: u64,
}

impl MutualCreditSystem {
    /// Create a new mutual credit system
    pub fn new(config: MutualCreditConfig) -> Self {
        Self {
            config,
            balances: HashMap::new(),
            transactions: HashMap::new(),
        }
    }
}

/// Mutual credit ledger interface
pub struct MutualCreditLedger {
    /// The underlying mutual credit system
    system: MutualCreditSystem,
}

impl MutualCreditLedger {
    /// Create a new mutual credit ledger
    pub fn new(config: MutualCreditConfig) -> Self {
        Self {
            system: MutualCreditSystem::new(config),
        }
    }
} 