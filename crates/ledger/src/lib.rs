//! Mutual Credit Ledger for ICN
//!
//! This module provides a mutual credit ledger for the InterCooperative Network,
//! tracking credits, debits, and account balances between network participants.

use std::collections::HashMap;
use std::sync::Arc;
use std::fmt;

use async_trait::async_trait;
use tokio::sync::RwLock;
use thiserror::Error;
use serde::{Serialize, Deserialize};

use icn_core::{
    storage::{Storage, StorageResult, StorageError, JsonStorage},
    crypto::{NodeId, Signature, Hash, sha256},
    utils::timestamp_secs,
};

use icn_identity::{
    Identity, IdentityProvider, IdentityError,
};

/// Error types for ledger operations
#[derive(Error, Debug)]
pub enum LedgerError {
    /// Error with the identity system
    #[error("Identity error: {0}")]
    IdentityError(#[from] IdentityError),
    
    /// Error with storage
    #[error("Storage error: {0}")]
    StorageError(#[from] StorageError),
    
    /// Invalid transaction
    #[error("Invalid transaction: {0}")]
    InvalidTransaction(String),
    
    /// Account not found
    #[error("Account not found: {0}")]
    AccountNotFound(String),
    
    /// Transaction not found
    #[error("Transaction not found: {0}")]
    TransactionNotFound(String),
    
    /// Insufficient balance
    #[error("Insufficient balance: {0}")]
    InsufficientBalance(String),
    
    /// Credit limit exceeded
    #[error("Credit limit exceeded: {0}")]
    CreditLimitExceeded(String),
    
    /// Permission denied
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    
    /// IO error
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),
}

/// Result type for ledger operations
pub type LedgerResult<T> = Result<T, LedgerError>;

/// Status of a transaction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionStatus {
    /// Pending, not yet confirmed
    Pending,
    /// Confirmed and recorded in the ledger
    Confirmed,
    /// Rejected due to validation failure
    Rejected,
    /// Cancelled by the initiator
    Cancelled,
    /// Failed during processing
    Failed,
}

impl Default for TransactionStatus {
    fn default() -> Self {
        Self::Pending
    }
}

/// Types of transactions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionType {
    /// Transfer of credits between accounts
    Transfer,
    /// Initial issuance of credits (system transaction)
    Issuance,
    /// Clearing of mutual debt
    Clearing,
    /// Account creation
    AccountCreation,
    /// Account update
    AccountUpdate,
    /// Credit limit adjustment
    CreditLimitAdjustment,
    /// Custom transaction type
    Custom(String),
}

impl Default for TransactionType {
    fn default() -> Self {
        Self::Transfer
    }
}

/// A transaction in the ledger
#[derive(Clone, Serialize, Deserialize)]
pub struct Transaction {
    /// Unique identifier for this transaction
    pub id: String,
    /// The type of transaction
    pub transaction_type: TransactionType,
    /// The account that initiated the transaction
    pub from_account: String,
    /// The recipient account (if applicable)
    pub to_account: Option<String>,
    /// The amount of the transaction
    pub amount: f64,
    /// The currency or unit of account
    pub currency: String,
    /// Description of the transaction
    pub description: String,
    /// Additional metadata for the transaction
    pub metadata: HashMap<String, String>,
    /// When the transaction was created
    pub created_at: u64,
    /// When the transaction was confirmed or finalized
    pub confirmed_at: Option<u64>,
    /// The current status of the transaction
    pub status: TransactionStatus,
    /// Reference to previous transactions (e.g., for clearing)
    pub references: Vec<String>,
    /// The signature from the initiator
    pub signature: Signature,
    /// The signature from the recipient (for mutual confirmation)
    pub counter_signature: Option<Signature>,
}

impl Transaction {
    /// Create a new unsigned transaction
    pub fn new(
        transaction_type: TransactionType,
        from_account: String,
        to_account: Option<String>,
        amount: f64,
        currency: String,
        description: String,
        metadata: HashMap<String, String>,
        references: Vec<String>,
    ) -> Self {
        let created_at = timestamp_secs();
        let id = format!("tx-{}-{}", from_account, created_at);
        
        Self {
            id,
            transaction_type,
            from_account,
            to_account,
            amount,
            currency,
            description,
            metadata,
            created_at,
            confirmed_at: None,
            status: TransactionStatus::Pending,
            references,
            signature: Signature(Vec::new()), // Placeholder, will be set when signed
            counter_signature: None,
        }
    }
    
    /// Get the bytes to sign for this transaction
    pub fn bytes_to_sign(&self) -> Vec<u8> {
        // Serialize the transaction data without the signatures
        let serializable = TransactionData {
            id: self.id.clone(),
            transaction_type: self.transaction_type.clone(),
            from_account: self.from_account.clone(),
            to_account: self.to_account.clone(),
            amount: self.amount,
            currency: self.currency.clone(),
            description: self.description.clone(),
            metadata: self.metadata.clone(),
            created_at: self.created_at,
            confirmed_at: self.confirmed_at,
            status: self.status,
            references: self.references.clone(),
        };
        
        serde_json::to_vec(&serializable).unwrap_or_default()
    }
    
    /// Check if this transaction is a valid transfer
    pub fn is_valid_transfer(&self) -> bool {
        // A transfer must have a recipient, a positive amount, and a from_account != to_account
        if self.transaction_type != TransactionType::Transfer {
            return false;
        }
        
        if self.to_account.is_none() {
            return false;
        }
        
        if self.amount <= 0.0 {
            return false;
        }
        
        if let Some(to) = &self.to_account {
            if to == &self.from_account {
                return false;
            }
        }
        
        true
    }
}

impl fmt::Debug for Transaction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Transaction {{ id: {}, type: {:?}, from: {}, to: {:?}, amount: {} {}, status: {:?} }}",
            self.id, self.transaction_type, self.from_account, self.to_account, self.amount, self.currency, self.status)
    }
}

/// Serializable transaction data for signing
#[derive(Serialize, Deserialize)]
struct TransactionData {
    /// Unique identifier for this transaction
    pub id: String,
    /// The type of transaction
    pub transaction_type: TransactionType,
    /// The account that initiated the transaction
    pub from_account: String,
    /// The recipient account (if applicable)
    pub to_account: Option<String>,
    /// The amount of the transaction
    pub amount: f64,
    /// The currency or unit of account
    pub currency: String,
    /// Description of the transaction
    pub description: String,
    /// Additional metadata for the transaction
    pub metadata: HashMap<String, String>,
    /// When the transaction was created
    pub created_at: u64,
    /// When the transaction was confirmed or finalized
    pub confirmed_at: Option<u64>,
    /// The current status of the transaction
    pub status: TransactionStatus,
    /// Reference to previous transactions (e.g., for clearing)
    pub references: Vec<String>,
}

/// An account in the ledger
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    /// Unique identifier for this account
    pub id: String,
    /// The owner's identity ID
    pub owner_id: NodeId,
    /// The name or label for the account
    pub name: String,
    /// The currency or unit of account
    pub currency: String,
    /// The current balance
    pub balance: f64,
    /// The credit limit (how negative the balance can go)
    pub credit_limit: f64,
    /// The last transaction ID
    pub last_transaction_id: Option<String>,
    /// When the account was created
    pub created_at: u64,
    /// When the account was last updated
    pub updated_at: u64,
    /// Additional metadata for the account
    pub metadata: HashMap<String, String>,
}

impl Account {
    /// Create a new account
    pub fn new(
        owner_id: NodeId,
        name: String,
        currency: String,
        credit_limit: f64,
        metadata: HashMap<String, String>,
    ) -> Self {
        let id = format!("account-{}-{}", owner_id, currency);
        let now = timestamp_secs();
        
        Self {
            id,
            owner_id,
            name,
            currency,
            balance: 0.0,
            credit_limit,
            last_transaction_id: None,
            created_at: now,
            updated_at: now,
            metadata,
        }
    }
    
    /// Check if a debit is within the account's credit limit
    pub fn can_debit(&self, amount: f64) -> bool {
        // Calculate the new balance after the debit
        let new_balance = self.balance - amount;
        
        // Check if it would exceed the credit limit
        new_balance >= -self.credit_limit
    }
    
    /// Apply a credit (increase balance)
    pub fn apply_credit(&mut self, amount: f64, transaction_id: &str) {
        self.balance += amount;
        self.last_transaction_id = Some(transaction_id.to_string());
        self.updated_at = timestamp_secs();
    }
    
    /// Apply a debit (decrease balance)
    pub fn apply_debit(&mut self, amount: f64, transaction_id: &str) -> LedgerResult<()> {
        if !self.can_debit(amount) {
            return Err(LedgerError::CreditLimitExceeded(
                format!("Debit of {} would exceed credit limit of {}", amount, self.credit_limit)
            ));
        }
        
        self.balance -= amount;
        self.last_transaction_id = Some(transaction_id.to_string());
        self.updated_at = timestamp_secs();
        
        Ok(())
    }
    
    /// Update the credit limit
    pub fn update_credit_limit(&mut self, new_limit: f64) -> LedgerResult<()> {
        // Check if the new limit would put the account over limit
        if -self.balance > new_limit {
            return Err(LedgerError::CreditLimitExceeded(
                format!("Current balance ({}) exceeds new credit limit ({})", self.balance, new_limit)
            ));
        }
        
        self.credit_limit = new_limit;
        self.updated_at = timestamp_secs();
        
        Ok(())
    }
}

/// Configuration for the ledger
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerConfig {
    /// Default credit limit for new accounts
    pub default_credit_limit: f64,
    /// Default currency
    pub default_currency: String,
    /// Whether transactions require counter-signatures
    pub require_counter_signatures: bool,
    /// Whether to automatically clear mutual debt
    pub auto_clear_mutual_debt: bool,
    /// Maximum transaction amount
    pub max_transaction_amount: f64,
    /// Custom configuration
    pub custom: HashMap<String, String>,
}

impl Default for LedgerConfig {
    fn default() -> Self {
        Self {
            default_credit_limit: 100.0,
            default_currency: "CRED".to_string(),
            require_counter_signatures: true,
            auto_clear_mutual_debt: true,
            max_transaction_amount: 1000.0,
            custom: HashMap::new(),
        }
    }
}

/// A trait for ledger operations
#[async_trait]
pub trait Ledger: Send + Sync {
    /// Get the ledger configuration
    async fn get_config(&self) -> LedgerResult<LedgerConfig>;
    
    /// Set the ledger configuration
    async fn set_config(&self, config: LedgerConfig) -> LedgerResult<()>;
    
    /// Create a new account
    async fn create_account(
        &self,
        name: String,
        currency: Option<String>,
        credit_limit: Option<f64>,
        metadata: HashMap<String, String>,
    ) -> LedgerResult<Account>;
    
    /// Get an account by ID
    async fn get_account(&self, id: &str) -> LedgerResult<Option<Account>>;
    
    /// Get accounts owned by an identity
    async fn get_accounts_by_owner(&self, owner_id: &NodeId) -> LedgerResult<Vec<Account>>;
    
    /// Update an account's metadata
    async fn update_account_metadata(
        &self,
        account_id: &str,
        metadata: HashMap<String, String>,
    ) -> LedgerResult<Account>;
    
    /// Update an account's credit limit
    async fn update_credit_limit(
        &self,
        account_id: &str,
        new_limit: f64,
    ) -> LedgerResult<Account>;
    
    /// Create a new transaction
    async fn create_transaction(
        &self,
        transaction_type: TransactionType,
        from_account: &str,
        to_account: Option<&str>,
        amount: f64,
        currency: Option<String>,
        description: String,
        metadata: HashMap<String, String>,
        references: Vec<String>,
    ) -> LedgerResult<Transaction>;
    
    /// Get a transaction by ID
    async fn get_transaction(&self, id: &str) -> LedgerResult<Option<Transaction>>;
    
    /// Get transactions by account
    async fn get_transactions_by_account(
        &self,
        account_id: &str,
    ) -> LedgerResult<Vec<Transaction>>;
    
    /// Counter-sign a transaction (recipient approval)
    async fn counter_sign_transaction(&self, id: &str) -> LedgerResult<Transaction>;
    
    /// Confirm a transaction (process it and update balances)
    async fn confirm_transaction(&self, id: &str) -> LedgerResult<Transaction>;
    
    /// Cancel a transaction (only allowed by the initiator and for pending transactions)
    async fn cancel_transaction(&self, id: &str) -> LedgerResult<Transaction>;
    
    /// Get the current balance of an account
    async fn get_balance(&self, account_id: &str) -> LedgerResult<f64>;
    
    /// Clear mutual debt between two accounts
    async fn clear_mutual_debt(
        &self,
        account1_id: &str,
        account2_id: &str,
    ) -> LedgerResult<Option<Transaction>>;
}

pub mod mutual_credit;
pub mod transaction_processor;
pub mod account_manager;

// Re-exports
pub use mutual_credit::MutualCreditLedger;
pub use transaction_processor::TransactionProcessor;
pub use account_manager::AccountManager; 