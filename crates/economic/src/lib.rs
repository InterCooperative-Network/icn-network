//! Intercooperative Economic System
//!
//! This module provides the economic functionality for the ICN network,
//! including mutual credit, incentives, and tokenized economic transactions.

use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;
use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

// Re-export the core types from icn-mutual-credit
pub use icn_mutual_credit::{
    Account,
    AccountId,
    Amount,
    CreditLimit,
    MutualCreditSystem,
    Transaction,
    TransactionId,
    TransactionStatus,
    TransactionType
};

pub mod incentives;
pub mod resource;
pub mod ledger;
pub mod factory;

// Re-export common types
pub use resource::{
    ResourceType,
    ResourceConfig,
    Resource,
    ResourceQuota,
};

pub use ledger::{
    MutualCreditLedger,
    LegacyMutualCreditSystem,
    AccountBalance,
    Transaction,
    AccountManager,
    Account,
    TransactionProcessor,
};

// Re-export factory
pub use factory::MutualCreditFactory;

// Re-export new implementation
pub use ledger::{
    MutualCreditSystem,
    MutualCreditConfig,
};

/// Economic error types
#[derive(Debug, thiserror::Error)]
pub enum EconomicError {
    #[error("Insufficient funds: {0}")]
    InsufficientFunds(String),
    
    #[error("Account not found: {0}")]
    AccountNotFound(String),
    
    #[error("Transaction not found: {0}")]
    TransactionNotFound(String),
    
    #[error("Invalid transaction: {0}")]
    InvalidTransaction(String),
    
    #[error("Credit limit exceeded: {0}")]
    CreditLimitExceeded(String),
    
    #[error("Storage error: {0}")]
    Storage(String),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Internal error: {0}")]
    Internal(String),
    
    #[error("Dependency error: {0}")]
    Dependency(String),
}

pub type Result<T> = std::result::Result<T, EconomicError>;

/// Federation economic configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationEconomicConfig {
    /// Federation ID
    pub federation_id: String,
    
    /// Name of the economic system
    pub name: String,
    
    /// Description
    pub description: String,
    
    /// Credit limit for new members
    pub default_credit_limit: i64,
    
    /// Transaction fee percentage
    pub fee_percentage: f64,
    
    /// Fee recipient account
    pub fee_recipient: Option<String>,
    
    /// Whether democratic approval is required for credit limit increases
    pub democratic_credit_approval: bool,
    
    /// Maximum credit limit without approval
    pub max_automatic_credit_limit: i64,
    
    /// Metadata
    pub metadata: HashMap<String, String>,
}

impl Default for FederationEconomicConfig {
    fn default() -> Self {
        Self {
            federation_id: "default".to_string(),
            name: "Default Economic System".to_string(),
            description: "Default mutual credit system for ICN".to_string(),
            default_credit_limit: 1000,
            fee_percentage: 0.0,
            fee_recipient: None,
            democratic_credit_approval: true,
            max_automatic_credit_limit: 5000,
            metadata: HashMap::new(),
        }
    }
}

/// Economic system for ICN
pub struct EconomicSystem {
    /// Mutual credit system
    mutual_credit: MutualCreditSystem,
    
    /// Federation configuration
    config: FederationEconomicConfig,
}

impl EconomicSystem {
    /// Create a new economic system with the specified configuration
    pub fn new(config: FederationEconomicConfig) -> Self {
        info!("Initializing economic system for federation: {}", config.federation_id);
        Self {
            mutual_credit: MutualCreditSystem::new(),
            config,
        }
    }
    
    /// Create a new account
    pub async fn create_account(&self, id: &str, name: &str) -> Result<Account> {
        let credit_limit = CreditLimit::new(self.config.default_credit_limit);
        
        info!("Creating account {} ({}) with credit limit {}", id, name, self.config.default_credit_limit);
        
        match self.mutual_credit.create_account(id.to_string(), name.to_string(), credit_limit) {
            Ok(account) => Ok(account),
            Err(e) => {
                error!("Failed to create account: {}", e);
                Err(EconomicError::Internal(e.to_string()))
            }
        }
    }
    
    /// Get account information
    pub async fn get_account(&self, id: &str) -> Result<Account> {
        match self.mutual_credit.get_account(&id.to_string()) {
            Ok(account) => Ok(account),
            Err(e) => {
                error!("Failed to get account {}: {}", id, e);
                Err(EconomicError::AccountNotFound(id.to_string()))
            }
        }
    }
    
    /// Create a transaction between accounts
    pub async fn create_transaction(
        &self,
        source_account: &str,
        destination_account: &str,
        amount: i64,
        description: &str,
        metadata: Option<serde_json::Value>,
    ) -> Result<Transaction> {
        let amount = Amount::new(amount);
        
        debug!(
            "Creating transaction from {} to {} for amount {}: {}",
            source_account, destination_account, amount.value(), description
        );
        
        match self.mutual_credit.create_transaction(
            source_account.to_string(),
            destination_account.to_string(),
            amount,
            description.to_string(),
            metadata,
        ) {
            Ok(tx) => Ok(tx),
            Err(e) => {
                error!("Failed to create transaction: {}", e);
                match e.to_string() {
                    s if s.contains("credit limit") => Err(EconomicError::CreditLimitExceeded(s)),
                    s if s.contains("account not found") => Err(EconomicError::AccountNotFound(s)),
                    _ => Err(EconomicError::Internal(e.to_string())),
                }
            }
        }
    }
    
    /// Execute a pending transaction
    pub async fn execute_transaction(&self, transaction_id: &str) -> Result<Transaction> {
        info!("Executing transaction: {}", transaction_id);
        
        match self.mutual_credit.execute_transaction(&transaction_id.to_string()) {
            Ok(tx) => Ok(tx),
            Err(e) => {
                error!("Failed to execute transaction {}: {}", transaction_id, e);
                match e.to_string() {
                    s if s.contains("transaction not found") => Err(EconomicError::TransactionNotFound(s)),
                    s if s.contains("insufficient funds") => Err(EconomicError::InsufficientFunds(s)),
                    s if s.contains("credit limit") => Err(EconomicError::CreditLimitExceeded(s)),
                    _ => Err(EconomicError::Internal(e.to_string())),
                }
            }
        }
    }
    
    /// Get transaction details
    pub async fn get_transaction(&self, id: &str) -> Result<Transaction> {
        match self.mutual_credit.get_transaction(&id.to_string()) {
            Ok(tx) => Ok(tx),
            Err(e) => {
                error!("Failed to get transaction {}: {}", id, e);
                Err(EconomicError::TransactionNotFound(id.to_string()))
            }
        }
    }
    
    /// Get account balance
    pub async fn get_balance(&self, account_id: &str) -> Result<Amount> {
        match self.mutual_credit.get_account_balance(&account_id.to_string()) {
            Ok(balance) => Ok(balance),
            Err(e) => {
                error!("Failed to get balance for account {}: {}", account_id, e);
                Err(EconomicError::AccountNotFound(account_id.to_string()))
            }
        }
    }
    
    /// Update credit limit
    pub async fn update_credit_limit(&self, account_id: &str, credit_limit: i64) -> Result<Account> {
        let limit = CreditLimit::new(credit_limit);
        
        // Check if democratic approval is required
        if self.config.democratic_credit_approval && credit_limit > self.config.max_automatic_credit_limit {
            error!("Credit limit increase exceeds automatic limit and requires governance approval");
            return Err(EconomicError::InvalidTransaction(
                "Credit limit increase requires governance approval".to_string()
            ));
        }
        
        info!("Updating credit limit for account {} to {}", account_id, credit_limit);
        
        match self.mutual_credit.update_credit_limit(&account_id.to_string(), limit) {
            Ok(account) => Ok(account),
            Err(e) => {
                error!("Failed to update credit limit: {}", e);
                Err(EconomicError::Internal(e.to_string()))
            }
        }
    }
    
    /// Get the current configuration
    pub fn get_config(&self) -> &FederationEconomicConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_create_account() {
        let config = FederationEconomicConfig::default();
        let system = EconomicSystem::new(config);
        
        let account = system.create_account("test1", "Test Account 1").await.unwrap();
        assert_eq!(account.id, "test1");
        assert_eq!(account.name, "Test Account 1");
        assert_eq!(account.credit_limit.value(), 1000);
    }
    
    #[tokio::test]
    async fn test_create_transaction() {
        let config = FederationEconomicConfig::default();
        let system = EconomicSystem::new(config);
        
        // Create accounts
        system.create_account("test1", "Test Account 1").await.unwrap();
        system.create_account("test2", "Test Account 2").await.unwrap();
        
        // Create transaction
        let tx = system.create_transaction(
            "test1",
            "test2",
            100,
            "Test transaction",
            None,
        ).await.unwrap();
        
        assert_eq!(tx.source_account, "test1");
        assert_eq!(tx.destination_account, "test2");
        assert_eq!(tx.amount.value(), 100);
        assert_eq!(tx.description, "Test transaction");
    }
} 