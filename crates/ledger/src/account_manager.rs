//! Account manager for ledger accounts
//!
//! This module provides functionality for creating and managing accounts
//! in the mutual credit ledger.

use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, info, warn};
use serde::{Serialize, Deserialize};

use icn_core::{
    storage::{Storage, JsonStorage},
    crypto::NodeId,
    utils::timestamp_secs,
};

use icn_identity::{
    IdentityProvider, IdentityResult, Identity,
};

use crate::{
    LedgerConfig, LedgerResult, LedgerError,
    Account,
};

/// The account manager for handling ledger accounts
pub struct AccountManager {
    /// Identity provider for authentication
    identity_provider: Arc<dyn IdentityProvider>,
    /// Storage for account data
    storage: Arc<dyn Storage>,
    /// Configuration
    config: LedgerConfig,
}

impl AccountManager {
    /// Create a new account manager
    pub fn new(
        identity_provider: Arc<dyn IdentityProvider>,
        storage: Arc<dyn Storage>,
        config: LedgerConfig,
    ) -> Self {
        Self {
            identity_provider,
            storage,
            config,
        }
    }
    
    /// Create a new account
    pub async fn create_account(
        &self,
        name: String,
        currency: Option<String>,
        credit_limit: Option<f64>,
        metadata: HashMap<String, String>,
    ) -> LedgerResult<Account> {
        // Get the current identity
        let identity = self.identity_provider.get_identity().await?;
        
        // Use defaults from config if not provided
        let currency = currency.unwrap_or_else(|| self.config.default_currency.clone());
        let credit_limit = credit_limit.unwrap_or(self.config.default_credit_limit);
        
        // Create the account
        let account = Account::new(
            identity.id.clone(),
            name,
            currency,
            credit_limit,
            metadata,
        );
        
        info!("Created new account: {}", account.id);
        
        Ok(account)
    }
    
    /// Validate account parameters
    pub fn validate_account_params(
        &self,
        currency: &str,
        credit_limit: f64,
    ) -> LedgerResult<()> {
        // Validate currency (simple validation, can be expanded)
        if currency.trim().is_empty() {
            return Err(LedgerError::InvalidTransaction(
                "Currency cannot be empty".to_string()
            ));
        }
        
        // Validate credit limit
        if credit_limit < 0.0 {
            return Err(LedgerError::InvalidTransaction(
                "Credit limit cannot be negative".to_string()
            ));
        }
        
        Ok(())
    }
    
    /// Get all unique currencies in use
    pub async fn get_currencies(&self, accounts: &HashMap<String, Account>) -> Vec<String> {
        let mut currencies = HashMap::new();
        
        for account in accounts.values() {
            currencies.insert(account.currency.clone(), true);
        }
        
        currencies.keys().cloned().collect()
    }
    
    /// Calculate the total balance for an owner across all accounts
    pub async fn calculate_total_balance(
        &self,
        owner_id: &NodeId,
        accounts: &HashMap<String, Account>,
    ) -> HashMap<String, f64> {
        let mut totals = HashMap::new();
        
        for account in accounts.values() {
            if account.owner_id == *owner_id {
                let entry = totals.entry(account.currency.clone()).or_insert(0.0);
                *entry += account.balance;
            }
        }
        
        totals
    }
    
    /// Find accounts by currency
    pub async fn find_accounts_by_currency(
        &self,
        currency: &str,
        accounts: &HashMap<String, Account>,
    ) -> Vec<Account> {
        let mut result = Vec::new();
        
        for account in accounts.values() {
            if account.currency == currency {
                result.push(account.clone());
            }
        }
        
        result
    }
    
    /// Find account by owner and currency
    pub async fn find_account_by_owner_and_currency(
        &self,
        owner_id: &NodeId,
        currency: &str,
        accounts: &HashMap<String, Account>,
    ) -> Option<Account> {
        for account in accounts.values() {
            if account.owner_id == *owner_id && account.currency == currency {
                return Some(account.clone());
            }
        }
        
        None
    }
    
    /// Get accounts with negative balances
    pub async fn get_accounts_with_negative_balance(
        &self,
        accounts: &HashMap<String, Account>,
    ) -> Vec<Account> {
        let mut result = Vec::new();
        
        for account in accounts.values() {
            if account.balance < 0.0 {
                result.push(account.clone());
            }
        }
        
        result
    }
    
    /// Get accounts approaching their credit limit
    pub async fn get_accounts_approaching_limit(
        &self,
        accounts: &HashMap<String, Account>,
        threshold_percentage: f64,
    ) -> Vec<Account> {
        let mut result = Vec::new();
        
        for account in accounts.values() {
            if account.balance < 0.0 {
                let used_percentage = -account.balance / account.credit_limit;
                if used_percentage >= threshold_percentage {
                    result.push(account.clone());
                }
            }
        }
        
        result
    }
    
    /// Generate account statistics
    pub async fn generate_account_statistics(
        &self,
        accounts: &HashMap<String, Account>,
    ) -> AccountStatistics {
        let mut total_accounts = 0;
        let mut total_positive_balance = 0.0;
        let mut total_negative_balance = 0.0;
        let mut by_currency = HashMap::new();
        
        for account in accounts.values() {
            total_accounts += 1;
            
            if account.balance > 0.0 {
                total_positive_balance += account.balance;
            } else if account.balance < 0.0 {
                total_negative_balance += account.balance;
            }
            
            // Add to currency-specific stats
            let currency_stats = by_currency
                .entry(account.currency.clone())
                .or_insert_with(CurrencyStatistics::default);
            
            currency_stats.account_count += 1;
            currency_stats.total_balance += account.balance;
            currency_stats.total_credit_limit += account.credit_limit;
            
            if account.balance > 0.0 {
                currency_stats.positive_balance_count += 1;
                currency_stats.total_positive_balance += account.balance;
            } else if account.balance < 0.0 {
                currency_stats.negative_balance_count += 1;
                currency_stats.total_negative_balance += account.balance;
            } else {
                currency_stats.zero_balance_count += 1;
            }
        }
        
        AccountStatistics {
            total_accounts,
            total_positive_balance,
            total_negative_balance,
            zero_sum_delta: total_positive_balance + total_negative_balance,
            by_currency,
        }
    }
}

/// Statistics for a specific currency
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CurrencyStatistics {
    /// Number of accounts in this currency
    pub account_count: usize,
    /// Total balance across all accounts in this currency
    pub total_balance: f64,
    /// Total credit limit across all accounts in this currency
    pub total_credit_limit: f64,
    /// Number of accounts with positive balance
    pub positive_balance_count: usize,
    /// Number of accounts with negative balance
    pub negative_balance_count: usize,
    /// Number of accounts with zero balance
    pub zero_balance_count: usize,
    /// Total positive balance
    pub total_positive_balance: f64,
    /// Total negative balance
    pub total_negative_balance: f64,
}

/// Statistics for all accounts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountStatistics {
    /// Total number of accounts
    pub total_accounts: usize,
    /// Total positive balance across all currencies
    pub total_positive_balance: f64,
    /// Total negative balance across all currencies
    pub total_negative_balance: f64,
    /// Delta between positive and negative balances (should be close to zero)
    pub zero_sum_delta: f64,
    /// Statistics by currency
    pub by_currency: HashMap<String, CurrencyStatistics>,
} 