use std::collections::HashMap;
use serde::{Serialize, Deserialize};

/// Account structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    /// Unique ID
    pub id: String,
    /// Account name
    pub name: String,
    /// Owner ID
    pub owner_id: String,
    /// Creation timestamp
    pub created_at: u64,
    /// Current balance
    pub balance: f64,
    /// Credit limit
    pub credit_limit: f64,
    /// Currency code
    pub currency: String,
    /// Account status
    pub status: AccountStatus,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Account status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AccountStatus {
    /// Account is active
    Active,
    /// Account is frozen
    Frozen,
    /// Account is closed
    Closed,
}

/// Account manager for handling account operations
pub struct AccountManager {
    /// Accounts by ID
    accounts: HashMap<String, Account>,
}

impl AccountManager {
    /// Create a new account manager
    pub fn new() -> Self {
        Self {
            accounts: HashMap::new(),
        }
    }
    
    /// Get an account by ID
    pub fn get_account(&self, id: &str) -> Option<&Account> {
        self.accounts.get(id)
    }
    
    /// Create a new account
    pub fn create_account(
        &mut self,
        name: String,
        owner_id: String,
        currency: String,
        credit_limit: f64,
        metadata: HashMap<String, String>,
    ) -> Account {
        let id = format!("acc-{}", rand::random::<u64>());
        let account = Account {
            id: id.clone(),
            name,
            owner_id,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            balance: 0.0,
            credit_limit,
            currency,
            status: AccountStatus::Active,
            metadata,
        };
        
        self.accounts.insert(id.clone(), account.clone());
        account
    }
} 