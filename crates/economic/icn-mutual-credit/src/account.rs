//! Account management for the mutual credit system.

use crate::types::{Amount, DID, Timestamp};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Status of an account
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccountStatus {
    /// Account is active
    Active,
    /// Account is inactive
    Inactive,
    /// Account is suspended
    Suspended,
    /// Account is closed
    Closed,
}

/// An account in the mutual credit system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    /// Decentralized identifier for the account
    pub did: DID,
    /// Human-readable name for the account
    pub name: String,
    /// Current status of the account
    pub status: AccountStatus,
    /// When the account was created
    pub created_at: Timestamp,
    /// When the account was last updated
    pub updated_at: Timestamp,
    /// Current balance of the account
    pub balance: Amount,
    /// Reputation score of the account (0.0 to 1.0)
    pub reputation: f64,
    /// Additional metadata for the account
    pub metadata: HashMap<String, String>,
}

impl Account {
    /// Create a new account
    pub fn new(did: DID, name: String) -> Self {
        let now = chrono::Utc::now();
        Self {
            did,
            name,
            status: AccountStatus::Active,
            created_at: now,
            updated_at: now,
            balance: Amount::zero(),
            reputation: 0.5, // Default neutral reputation
            metadata: HashMap::new(),
        }
    }

    /// Check if the account is active
    pub fn is_active(&self) -> bool {
        self.status == AccountStatus::Active
    }

    /// Update the account status
    pub fn update_status(&mut self, status: AccountStatus) {
        self.status = status;
        self.updated_at = chrono::Utc::now();
    }

    /// Update the account balance
    pub fn update_balance(&mut self, amount: Amount) {
        self.balance = self.balance.clone() + amount;
        self.updated_at = chrono::Utc::now();
    }

    /// Update the account reputation
    pub fn update_reputation(&mut self, reputation: f64) {
        // Ensure reputation is between 0.0 and 1.0
        self.reputation = reputation.max(0.0).min(1.0);
        self.updated_at = chrono::Utc::now();
    }

    /// Add metadata to the account
    pub fn add_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
        self.updated_at = chrono::Utc::now();
    }

    /// Get metadata from the account
    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }

    /// Suspend the account
    pub fn suspend(&mut self) {
        self.status = AccountStatus::Suspended;
        self.updated_at = chrono::Utc::now();
    }

    /// Reactivate the account
    pub fn reactivate(&mut self) {
        self.status = AccountStatus::Active;
        self.updated_at = chrono::Utc::now();
    }

    /// Close the account
    pub fn close(&mut self) {
        self.status = AccountStatus::Closed;
        self.updated_at = chrono::Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_account_basics() {
        let did = DID::new("test");
        let name = "Test Account".to_string();
        
        let mut account = Account::new(did.clone(), name.clone());
        
        assert_eq!(account.did, did);
        assert_eq!(account.name, name);
        assert_eq!(account.status, AccountStatus::Active);
        assert_eq!(account.balance, Amount::zero());
        assert_eq!(account.reputation, 0.5);
        assert!(account.metadata.is_empty());
        assert!(account.is_active());
        
        // Update balance
        let amount = Amount::new(100);
        account.update_balance(amount.clone());
        assert_eq!(account.balance, amount);
        
        // Update reputation
        account.update_reputation(0.8);
        assert_eq!(account.reputation, 0.8);
        
        // Add metadata
        account.add_metadata("location".to_string(), "New York".to_string());
        assert_eq!(account.metadata.len(), 1);
        assert_eq!(account.get_metadata("location"), Some(&"New York".to_string()));
    }

    #[test]
    fn test_account_status_changes() {
        let did = DID::new("test");
        let name = "Test Account".to_string();
        
        let mut account = Account::new(did, name);
        assert!(account.is_active());
        
        // Suspend the account
        account.suspend();
        assert_eq!(account.status, AccountStatus::Suspended);
        assert!(!account.is_active());
        
        // Reactivate the account
        account.reactivate();
        assert_eq!(account.status, AccountStatus::Active);
        assert!(account.is_active());
        
        // Close the account
        account.close();
        assert_eq!(account.status, AccountStatus::Closed);
        assert!(!account.is_active());
        
        // Update status directly
        account.update_status(AccountStatus::Inactive);
        assert_eq!(account.status, AccountStatus::Inactive);
    }

    #[test]
    fn test_account_reputation_bounds() {
        let did = DID::new("test");
        let name = "Test Account".to_string();
        
        let mut account = Account::new(did, name);
        
        // Test upper bound
        account.update_reputation(1.5);
        assert_eq!(account.reputation, 1.0);
        
        // Test lower bound
        account.update_reputation(-0.5);
        assert_eq!(account.reputation, 0.0);
        
        // Test valid value
        account.update_reputation(0.75);
        assert_eq!(account.reputation, 0.75);
    }
} 