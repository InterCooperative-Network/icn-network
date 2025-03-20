use async_trait::async_trait;
use std::error::Error;
use serde::{Serialize, Deserialize};

/// Account identifier
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct AccountId(pub String);

/// Transaction identifier
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct TransactionId(pub String);

/// Amount representation with currency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Amount {
    pub value: i64,
    pub currency: String,
}

/// Transaction status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TransactionStatus {
    Pending,
    Confirmed,
    Failed,
    Cancelled,
}

/// Transaction types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TransactionType {
    Transfer,
    CreditLimitChange,
    FeeCollection,
    ResourceExchange,
    Reward,
    Other(String),
}

/// Transaction representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: TransactionId,
    pub from: AccountId,
    pub to: AccountId,
    pub amount: Amount,
    pub timestamp: u64,
    pub transaction_type: TransactionType,
    pub status: TransactionStatus,
    pub metadata: std::collections::HashMap<String, String>,
    pub signature: Option<Vec<u8>>,
}

/// Account balance information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountBalance {
    pub account_id: AccountId,
    pub available: Amount,
    pub credit_limit: i64,
    pub reserved: i64,
    pub last_updated: u64,
}

/// Result type for economic operations
pub type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

/// Provider interface for economic-related operations
#[async_trait]
pub trait EconomicProvider: Send + Sync {
    /// Create a new account
    async fn create_account(&self, owner_did: &str, metadata: Option<std::collections::HashMap<String, String>>) -> Result<AccountId>;
    
    /// Get account balance
    async fn get_balance(&self, account_id: &AccountId) -> Result<AccountBalance>;
    
    /// Execute a transaction
    async fn execute_transaction(&self, transaction: Transaction) -> Result<TransactionStatus>;
    
    /// Get transaction by ID
    async fn get_transaction(&self, id: &TransactionId) -> Result<Option<Transaction>>;
    
    /// Get transactions for an account
    async fn get_account_transactions(&self, account_id: &AccountId, limit: Option<u64>, offset: Option<u64>) -> Result<Vec<Transaction>>;
    
    /// Update credit limit for an account
    async fn update_credit_limit(&self, account_id: &AccountId, new_limit: i64) -> Result<()>;
} 