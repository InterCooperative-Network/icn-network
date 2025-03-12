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
mod confidential;

pub use account::{Account, AccountStatus};
pub use credit_graph::{CreditGraph, CreditLineId, CreditLineStep};
pub use credit_line::{
    CollateralRequirement, CollateralType, CreditCondition, CreditLine, CreditTerms, ResourceCommitment,
};
pub use error::CreditError;
pub use transaction::{Transaction, TransactionStatus, TransactionType};
pub use transaction_processor::{TransactionProcessor, TransactionResult, CreditClearingParams};
pub use types::{Amount, DID, Timestamp};
pub use confidential::*;

/// Version of the mutual credit implementation
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

use icn_common::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Account identifier
pub type AccountId = String;

/// Transaction identifier
pub type TransactionId = String;

/// Amount type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Amount(i64);

impl Amount {
    /// Create a new amount
    pub fn new(value: i64) -> Self {
        Self(value)
    }
    
    /// Get the value
    pub fn value(&self) -> i64 {
        self.0
    }
    
    /// Check if the amount is positive
    pub fn is_positive(&self) -> bool {
        self.0 > 0
    }
    
    /// Check if the amount is negative
    pub fn is_negative(&self) -> bool {
        self.0 < 0
    }
    
    /// Check if the amount is zero
    pub fn is_zero(&self) -> bool {
        self.0 == 0
    }
    
    /// Add another amount
    pub fn add(&self, other: Amount) -> Amount {
        Amount(self.0 + other.0)
    }
    
    /// Subtract another amount
    pub fn subtract(&self, other: Amount) -> Amount {
        Amount(self.0 - other.0)
    }
    
    /// Negate the amount
    pub fn negate(&self) -> Amount {
        Amount(-self.0)
    }
    
    /// Get the absolute value
    pub fn abs(&self) -> Amount {
        Amount(self.0.abs())
    }
}

/// Credit limit
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CreditLimit(i64);

impl CreditLimit {
    /// Create a new credit limit
    pub fn new(value: i64) -> Self {
        // Credit limits should be positive
        Self(value.abs())
    }
    
    /// Get the value
    pub fn value(&self) -> i64 {
        self.0
    }
}

/// A mutual credit account
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    /// Account ID
    pub id: AccountId,
    
    /// Account name
    pub name: String,
    
    /// Account balance
    pub balance: Amount,
    
    /// Credit limit
    pub credit_limit: CreditLimit,
    
    /// Creation time
    pub created_at: DateTime<Utc>,
    
    /// Last update time
    pub updated_at: DateTime<Utc>,
}

impl Account {
    /// Create a new account
    pub fn new(id: AccountId, name: String, credit_limit: CreditLimit) -> Self {
        let now = Utc::now();
        Self {
            id,
            name,
            balance: Amount::new(0),
            credit_limit,
            created_at: now,
            updated_at: now,
        }
    }
    
    /// Check if a transaction is valid for this account
    pub fn can_transact(&self, amount: Amount) -> bool {
        // If amount is positive, we're receiving credit, so always allow
        if amount.is_positive() {
            return true;
        }
        
        // If amount is negative, check if we have enough credit
        let new_balance = self.balance.add(amount);
        new_balance.value() >= -self.credit_limit.value()
    }
    
    /// Apply a transaction to this account
    pub fn apply_transaction(&mut self, amount: Amount) -> Result<()> {
        if !self.can_transact(amount) {
            return Err(Error::validation(format!(
                "Insufficient credit for account {}: balance={}, limit={}, amount={}",
                self.id, self.balance.value(), self.credit_limit.value(), amount.value()
            )));
        }
        
        self.balance = self.balance.add(amount);
        self.updated_at = Utc::now();
        Ok(())
    }
}

/// Transaction status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionStatus {
    /// Transaction is pending
    Pending,
    
    /// Transaction is completed
    Completed,
    
    /// Transaction failed
    Failed,
    
    /// Transaction is cancelled
    Cancelled,
}

/// Transaction type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionType {
    /// Direct transfer between accounts
    Transfer,
    
    /// System adjustment (e.g., initial credit issuance)
    Adjustment,
    
    /// Fee payment
    Fee,
}

/// A mutual credit transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    /// Transaction ID
    pub id: TransactionId,
    
    /// Transaction type
    pub transaction_type: TransactionType,
    
    /// Source account
    pub source_account: AccountId,
    
    /// Destination account
    pub destination_account: AccountId,
    
    /// Amount
    pub amount: Amount,
    
    /// Status
    pub status: TransactionStatus,
    
    /// Description
    pub description: String,
    
    /// Creation time
    pub created_at: DateTime<Utc>,
    
    /// Completion time
    pub completed_at: Option<DateTime<Utc>>,
    
    /// Metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl Transaction {
    /// Create a new transaction
    pub fn new(
        source_account: AccountId,
        destination_account: AccountId,
        amount: Amount,
        transaction_type: TransactionType,
        description: String,
        metadata: Option<serde_json::Value>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            transaction_type,
            source_account,
            destination_account,
            amount,
            status: TransactionStatus::Pending,
            description,
            created_at: Utc::now(),
            completed_at: None,
            metadata,
        }
    }
    
    /// Mark the transaction as completed
    pub fn complete(&mut self) {
        self.status = TransactionStatus::Completed;
        self.completed_at = Some(Utc::now());
    }
    
    /// Mark the transaction as failed
    pub fn fail(&mut self) {
        self.status = TransactionStatus::Failed;
        self.completed_at = Some(Utc::now());
    }
    
    /// Mark the transaction as cancelled
    pub fn cancel(&mut self) {
        self.status = TransactionStatus::Cancelled;
        self.completed_at = Some(Utc::now());
    }
}

/// A simple mutual credit system
#[derive(Debug)]
pub struct MutualCreditSystem {
    /// Accounts
    accounts: RwLock<HashMap<AccountId, Account>>,
    
    /// Transactions
    transactions: RwLock<HashMap<TransactionId, Transaction>>,
}

impl MutualCreditSystem {
    /// Create a new mutual credit system
    pub fn new() -> Self {
        Self {
            accounts: RwLock::new(HashMap::new()),
            transactions: RwLock::new(HashMap::new()),
        }
    }
    
    /// Create a new account
    pub fn create_account(&self, id: AccountId, name: String, credit_limit: CreditLimit) -> Result<Account> {
        let mut accounts = self.accounts.write()
            .map_err(|_| Error::internal("Failed to acquire write lock on accounts"))?;
        
        if accounts.contains_key(&id) {
            return Err(Error::already_exists(format!("Account already exists: {}", id)));
        }
        
        let account = Account::new(id.clone(), name, credit_limit);
        accounts.insert(id, account.clone());
        
        Ok(account)
    }
    
    /// Get an account
    pub fn get_account(&self, id: &AccountId) -> Result<Account> {
        let accounts = self.accounts.read()
            .map_err(|_| Error::internal("Failed to acquire read lock on accounts"))?;
        
        accounts.get(id)
            .cloned()
            .ok_or_else(|| Error::not_found(format!("Account not found: {}", id)))
    }
    
    /// Create a new transaction
    pub fn create_transaction(
        &self,
        source_account: AccountId,
        destination_account: AccountId,
        amount: Amount,
        description: String,
        metadata: Option<serde_json::Value>,
    ) -> Result<Transaction> {
        // Validate accounts
        self.get_account(&source_account)?;
        self.get_account(&destination_account)?;
        
        // Validate amount
        if amount.is_zero() {
            return Err(Error::validation("Transaction amount cannot be zero"));
        }
        
        // Create the transaction
        let transaction = Transaction::new(
            source_account,
            destination_account,
            amount,
            TransactionType::Transfer,
            description,
            metadata,
        );
        
        // Store the transaction
        let mut transactions = self.transactions.write()
            .map_err(|_| Error::internal("Failed to acquire write lock on transactions"))?;
        
        transactions.insert(transaction.id.clone(), transaction.clone());
        
        Ok(transaction)
    }
    
    /// Execute a transaction
    pub fn execute_transaction(&self, transaction_id: &TransactionId) -> Result<Transaction> {
        // Get the transaction
        let mut transactions = self.transactions.write()
            .map_err(|_| Error::internal("Failed to acquire write lock on transactions"))?;
        
        let transaction = transactions.get_mut(transaction_id)
            .ok_or_else(|| Error::not_found(format!("Transaction not found: {}", transaction_id)))?;
        
        // Check if the transaction is already completed
        if transaction.status != TransactionStatus::Pending {
            return Err(Error::validation(format!(
                "Transaction is not pending: status={:?}",
                transaction.status
            )));
        }
        
        // Get the accounts
        let mut accounts = self.accounts.write()
            .map_err(|_| Error::internal("Failed to acquire write lock on accounts"))?;
        
        let source_account = accounts.get_mut(&transaction.source_account)
            .ok_or_else(|| Error::not_found(format!("Source account not found: {}", transaction.source_account)))?;
        
        let destination_account = accounts.get_mut(&transaction.destination_account)
            .ok_or_else(|| Error::not_found(format!("Destination account not found: {}", transaction.destination_account)))?;
        
        // Check if the source account has enough credit
        if !source_account.can_transact(transaction.amount.negate()) {
            transaction.fail();
            return Err(Error::validation(format!(
                "Insufficient credit for account {}: balance={}, limit={}, amount={}",
                source_account.id, source_account.balance.value(), source_account.credit_limit.value(), transaction.amount.value()
            )));
        }
        
        // Apply the transaction
        source_account.apply_transaction(transaction.amount.negate())?;
        destination_account.apply_transaction(transaction.amount)?;
        
        // Mark the transaction as completed
        transaction.complete();
        
        Ok(transaction.clone())
    }
    
    /// Get a transaction
    pub fn get_transaction(&self, id: &TransactionId) -> Result<Transaction> {
        let transactions = self.transactions.read()
            .map_err(|_| Error::internal("Failed to acquire read lock on transactions"))?;
        
        transactions.get(id)
            .cloned()
            .ok_or_else(|| Error::not_found(format!("Transaction not found: {}", id)))
    }
    
    /// Get all transactions for an account
    pub fn get_account_transactions(&self, account_id: &AccountId) -> Result<Vec<Transaction>> {
        let transactions = self.transactions.read()
            .map_err(|_| Error::internal("Failed to acquire read lock on transactions"))?;
        
        Ok(transactions.values()
            .filter(|t| t.source_account == *account_id || t.destination_account == *account_id)
            .cloned()
            .collect())
    }
    
    /// Calculate the net balance of an account
    pub fn get_account_balance(&self, account_id: &AccountId) -> Result<Amount> {
        let account = self.get_account(account_id)?;
        Ok(account.balance)
    }
    
    /// Update an account's credit limit
    pub fn update_credit_limit(&self, account_id: &AccountId, credit_limit: CreditLimit) -> Result<Account> {
        let mut accounts = self.accounts.write()
            .map_err(|_| Error::internal("Failed to acquire write lock on accounts"))?;
        
        let account = accounts.get_mut(account_id)
            .ok_or_else(|| Error::not_found(format!("Account not found: {}", account_id)))?;
        
        account.credit_limit = credit_limit;
        account.updated_at = Utc::now();
        
        Ok(account.clone())
    }
}

impl Default for MutualCreditSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_create_account() {
        let system = MutualCreditSystem::new();
        
        let account = system.create_account(
            "account1".to_string(),
            "Test Account".to_string(),
            CreditLimit::new(1000),
        ).unwrap();
        
        assert_eq!(account.id, "account1");
        assert_eq!(account.name, "Test Account");
        assert_eq!(account.balance.value(), 0);
        assert_eq!(account.credit_limit.value(), 1000);
    }
    
    #[test]
    fn test_create_transaction() {
        let system = MutualCreditSystem::new();
        
        // Create accounts
        system.create_account(
            "account1".to_string(),
            "Test Account 1".to_string(),
            CreditLimit::new(1000),
        ).unwrap();
        
        system.create_account(
            "account2".to_string(),
            "Test Account 2".to_string(),
            CreditLimit::new(1000),
        ).unwrap();
        
        // Create a transaction
        let transaction = system.create_transaction(
            "account1".to_string(),
            "account2".to_string(),
            Amount::new(500),
            "Test Transaction".to_string(),
            None,
        ).unwrap();
        
        assert_eq!(transaction.source_account, "account1");
        assert_eq!(transaction.destination_account, "account2");
        assert_eq!(transaction.amount.value(), 500);
        assert_eq!(transaction.status, TransactionStatus::Pending);
    }
    
    #[test]
    fn test_execute_transaction() {
        let system = MutualCreditSystem::new();
        
        // Create accounts
        system.create_account(
            "account1".to_string(),
            "Test Account 1".to_string(),
            CreditLimit::new(1000),
        ).unwrap();
        
        system.create_account(
            "account2".to_string(),
            "Test Account 2".to_string(),
            CreditLimit::new(1000),
        ).unwrap();
        
        // Create a transaction
        let transaction = system.create_transaction(
            "account1".to_string(),
            "account2".to_string(),
            Amount::new(500),
            "Test Transaction".to_string(),
            None,
        ).unwrap();
        
        // Execute the transaction
        system.execute_transaction(&transaction.id).unwrap();
        
        // Check account balances
        let account1 = system.get_account(&"account1".to_string()).unwrap();
        let account2 = system.get_account(&"account2".to_string()).unwrap();
        
        assert_eq!(account1.balance.value(), -500);
        assert_eq!(account2.balance.value(), 500);
    }
    
    #[test]
    fn test_credit_limit() {
        let system = MutualCreditSystem::new();
        
        // Create accounts
        system.create_account(
            "account1".to_string(),
            "Test Account 1".to_string(),
            CreditLimit::new(500),
        ).unwrap();
        
        system.create_account(
            "account2".to_string(),
            "Test Account 2".to_string(),
            CreditLimit::new(1000),
        ).unwrap();
        
        // Create a transaction
        let transaction = system.create_transaction(
            "account1".to_string(),
            "account2".to_string(),
            Amount::new(1000),
            "Test Transaction".to_string(),
            None,
        ).unwrap();
        
        // Execute the transaction should fail
        let result = system.execute_transaction(&transaction.id);
        assert!(result.is_err());
        
        // Check account balances (should be unchanged)
        let account1 = system.get_account(&"account1".to_string()).unwrap();
        let account2 = system.get_account(&"account2".to_string()).unwrap();
        
        assert_eq!(account1.balance.value(), 0);
        assert_eq!(account2.balance.value(), 0);
    }
} 