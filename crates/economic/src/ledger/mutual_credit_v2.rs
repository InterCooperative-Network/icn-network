use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use icn_core::interfaces::identity::IdentityProvider;
use icn_core::interfaces::storage::StorageProvider;
use icn_core::interfaces::reputation::ReputationProvider;
use icn_core::interfaces::economic::{
    EconomicProvider, AccountId, AccountBalance, Transaction, 
    TransactionId, TransactionStatus, Result
};
use tokio::sync::RwLock;
use tracing::{info, debug, error};
use serde::{Serialize, Deserialize};

/// Configuration for the mutual credit system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutualCreditConfig {
    /// Default credit limit
    pub default_credit_limit: i64,
    /// Transaction fee percentage
    pub fee_percentage: f64,
    /// Fee recipient account
    pub fee_recipient: Option<String>,
    /// System currency code
    pub currency_code: String,
    /// Base namespace for storage
    pub storage_namespace: String,
}

impl Default for MutualCreditConfig {
    fn default() -> Self {
        Self {
            default_credit_limit: 1000,
            fee_percentage: 0.0,
            fee_recipient: None,
            currency_code: "ICN".to_string(),
            storage_namespace: "mutual_credit".to_string(),
        }
    }
}

/// Improved MutualCreditSystem that uses dependency injection
pub struct MutualCreditSystem {
    config: MutualCreditConfig,
    identity_provider: Arc<dyn IdentityProvider>,
    storage_provider: Arc<dyn StorageProvider>,
    reputation_provider: Arc<dyn ReputationProvider>,
    
    // Cache for active accounts and transactions
    accounts: RwLock<HashMap<String, AccountBalance>>,
    transactions: RwLock<HashMap<String, Transaction>>,
}

impl MutualCreditSystem {
    /// Create a new MutualCreditSystem with the given dependencies
    pub fn new(
        config: MutualCreditConfig,
        identity_provider: Arc<dyn IdentityProvider>,
        storage_provider: Arc<dyn StorageProvider>,
        reputation_provider: Arc<dyn ReputationProvider>,
    ) -> Self {
        Self {
            config,
            identity_provider,
            storage_provider,
            reputation_provider,
            accounts: RwLock::new(HashMap::new()),
            transactions: RwLock::new(HashMap::new()),
        }
    }
    
    /// Initialize the system by loading data from storage
    pub async fn initialize(&self) -> Result<()> {
        debug!("Initializing MutualCreditSystem");
        
        // Load accounts and transactions from storage
        // Implementation omitted for brevity
        
        Ok(())
    }
    
    /// Internal helper to get account storage key
    fn get_account_key(&self, account_id: &AccountId) -> String {
        format!("{}:accounts:{}", self.config.storage_namespace, account_id.0)
    }
    
    /// Internal helper to get transaction storage key
    fn get_transaction_key(&self, tx_id: &TransactionId) -> String {
        format!("{}:transactions:{}", self.config.storage_namespace, tx_id.0)
    }
    
    /// Generate a new transaction ID
    fn generate_transaction_id(&self) -> TransactionId {
        use uuid::Uuid;
        TransactionId(format!("tx-{}", Uuid::new_v4()))
    }
    
    /// Generate a new account ID
    fn generate_account_id(&self, owner_did: &str) -> AccountId {
        use uuid::Uuid;
        AccountId(format!("acct-{}-{}", owner_did.split(':').last().unwrap_or("unknown"), Uuid::new_v4()))
    }
}

#[async_trait]
impl EconomicProvider for MutualCreditSystem {
    async fn create_account(&self, owner_did: &str, metadata: Option<HashMap<String, String>>) -> Result<AccountId> {
        // Validate the DID first using the identity provider
        let valid = self.identity_provider.validate_identity(owner_did).await?;
        if !valid {
            return Err("Invalid DID for account creation".into());
        }
        
        let account_id = self.generate_account_id(owner_did);
        
        // Initialize account balance
        let balance = AccountBalance {
            account_id: account_id.clone(),
            available: icn_core::interfaces::economic::Amount {
                value: 0,
                currency: self.config.currency_code.clone(),
            },
            credit_limit: self.config.default_credit_limit,
            reserved: 0,
            last_updated: chrono::Utc::now().timestamp() as u64,
        };
        
        // Store in both cache and persistent storage
        {
            let mut accounts = self.accounts.write().await;
            accounts.insert(account_id.0.clone(), balance.clone());
        }
        
        let storage_key = self.get_account_key(&account_id);
        self.storage_provider.store(&storage_key, &balance, None).await?;
        
        info!("Created new account: {}", account_id.0);
        Ok(account_id)
    }
    
    async fn get_balance(&self, account_id: &AccountId) -> Result<AccountBalance> {
        // Try to get from cache first
        {
            let accounts = self.accounts.read().await;
            if let Some(balance) = accounts.get(&account_id.0) {
                return Ok(balance.clone());
            }
        }
        
        // If not in cache, try to get from storage
        let storage_key = self.get_account_key(account_id);
        let balance = self.storage_provider.retrieve::<AccountBalance>(&storage_key, None).await?
            .ok_or_else(|| "Account not found".into())?;
        
        // Update cache
        {
            let mut accounts = self.accounts.write().await;
            accounts.insert(account_id.0.clone(), balance.clone());
        }
        
        Ok(balance)
    }
    
    async fn execute_transaction(&self, transaction: Transaction) -> Result<TransactionStatus> {
        // Implementation omitted for brevity
        // Would include:
        // 1. Validate transaction
        // 2. Check balances and credit limits
        // 3. Apply transaction
        // 4. Update storage
        // 5. Return updated status
        
        Ok(TransactionStatus::Confirmed)
    }
    
    async fn get_transaction(&self, id: &TransactionId) -> Result<Option<Transaction>> {
        // Try to get from cache first
        {
            let transactions = self.transactions.read().await;
            if let Some(tx) = transactions.get(&id.0) {
                return Ok(Some(tx.clone()));
            }
        }
        
        // If not in cache, try to get from storage
        let storage_key = self.get_transaction_key(id);
        let transaction = self.storage_provider.retrieve::<Transaction>(&storage_key, None).await?;
        
        // Update cache if found
        if let Some(tx) = &transaction {
            let mut transactions = self.transactions.write().await;
            transactions.insert(id.0.clone(), tx.clone());
        }
        
        Ok(transaction)
    }
    
    async fn get_account_transactions(&self, account_id: &AccountId, limit: Option<u64>, offset: Option<u64>) -> Result<Vec<Transaction>> {
        // Implementation omitted for brevity
        // Would search storage for transactions with matching account ID
        
        Ok(Vec::new())
    }
    
    async fn update_credit_limit(&self, account_id: &AccountId, new_limit: i64) -> Result<()> {
        // Validate the new credit limit
        if new_limit < 0 {
            return Err("Credit limit cannot be negative".into());
        }
        
        // Get the current balance
        let mut balance = self.get_balance(account_id).await?;
        
        // Update the credit limit
        balance.credit_limit = new_limit;
        balance.last_updated = chrono::Utc::now().timestamp() as u64;
        
        // Store the updated balance
        let storage_key = self.get_account_key(account_id);
        self.storage_provider.store(&storage_key, &balance, None).await?;
        
        // Update cache
        {
            let mut accounts = self.accounts.write().await;
            accounts.insert(account_id.0.clone(), balance);
        }
        
        info!("Updated credit limit for account {} to {}", account_id.0, new_limit);
        Ok(())
    }
} 