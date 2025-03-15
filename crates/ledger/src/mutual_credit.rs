//! Mutual Credit Ledger implementation
//!
//! This module provides an implementation of the Ledger trait
//! for mutual credit accounting.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use serde::{Serialize, Deserialize};
use async_trait::async_trait;

use icn_core::{
    storage::{Storage, JsonStorage},
    crypto::{NodeId, Signature},
    utils::timestamp_secs,
};

use icn_identity::{
    IdentityProvider, IdentityResult, Identity,
};

use crate::{
    Ledger, LedgerConfig, LedgerResult, LedgerError,
    Account, Transaction, TransactionStatus, TransactionType,
    transaction_processor::TransactionProcessor,
    account_manager::AccountManager,
};

/// Path constants for storage
const CONFIG_PATH: &str = "ledger/config";
const ACCOUNTS_PATH: &str = "ledger/accounts";
const TRANSACTIONS_PATH: &str = "ledger/transactions";
const ACCOUNT_TRANSACTIONS_PATH: &str = "ledger/account_transactions";

/// The main implementation of the Ledger trait
pub struct MutualCreditLedger {
    /// Identity provider for authentication and signatures
    identity_provider: Arc<dyn IdentityProvider>,
    /// Storage for ledger data
    storage: Arc<dyn Storage>,
    /// Current configuration
    config: Arc<RwLock<LedgerConfig>>,
    /// Accounts cache (by ID)
    accounts: Arc<RwLock<HashMap<String, Account>>>,
    /// Accounts by owner (owner_id -> set of account IDs)
    accounts_by_owner: Arc<RwLock<HashMap<String, HashSet<String>>>>,
    /// Transactions cache (by ID)
    transactions: Arc<RwLock<HashMap<String, Transaction>>>,
    /// Transactions by account (account_id -> set of transaction IDs)
    transactions_by_account: Arc<RwLock<HashMap<String, HashSet<String>>>>,
    /// Transaction processor for handling transactions
    transaction_processor: TransactionProcessor,
    /// Account manager for handling accounts
    account_manager: AccountManager,
}

impl MutualCreditLedger {
    /// Create a new mutual credit ledger
    pub async fn new(
        identity_provider: Arc<dyn IdentityProvider>,
        storage: Arc<dyn Storage>,
    ) -> LedgerResult<Self> {
        // Load configuration
        let config = Self::load_config(&storage).await?;
        
        let ledger = Self {
            identity_provider: identity_provider.clone(),
            storage: storage.clone(),
            config: Arc::new(RwLock::new(config.clone())),
            accounts: Arc::new(RwLock::new(HashMap::new())),
            accounts_by_owner: Arc::new(RwLock::new(HashMap::new())),
            transactions: Arc::new(RwLock::new(HashMap::new())),
            transactions_by_account: Arc::new(RwLock::new(HashMap::new())),
            transaction_processor: TransactionProcessor::new(
                identity_provider.clone(),
                storage.clone(),
                config.clone(),
            ),
            account_manager: AccountManager::new(
                identity_provider.clone(),
                storage.clone(),
                config.clone(),
            ),
        };
        
        // Load existing accounts and transactions
        ledger.load_accounts().await?;
        ledger.load_transactions().await?;
        
        Ok(ledger)
    }
    
    /// Load configuration from storage
    async fn load_config(storage: &dyn Storage) -> LedgerResult<LedgerConfig> {
        match storage.get_json::<LedgerConfig>(CONFIG_PATH).await {
            Ok(config) => Ok(config),
            Err(_) => {
                // If no config exists, use default and save it
                let config = LedgerConfig::default();
                if let Err(e) = storage.put_json(CONFIG_PATH, &config).await {
                    warn!("Failed to save default ledger configuration: {}", e);
                }
                Ok(config)
            }
        }
    }
    
    /// Load accounts from storage
    async fn load_accounts(&self) -> LedgerResult<()> {
        let account_keys = self.storage.list(ACCOUNTS_PATH).await?;
        let mut accounts = self.accounts.write().await;
        let mut accounts_by_owner = self.accounts_by_owner.write().await;
        
        for key in account_keys {
            match self.storage.get_json::<Account>(&key).await {
                Ok(account) => {
                    // Add to accounts cache
                    accounts.insert(account.id.clone(), account.clone());
                    
                    // Add to accounts by owner
                    let owner_id = account.owner_id.as_str().to_string();
                    let account_set = accounts_by_owner
                        .entry(owner_id)
                        .or_insert_with(HashSet::new);
                    account_set.insert(account.id.clone());
                },
                Err(e) => {
                    error!("Failed to load account {}: {}", key, e);
                }
            }
        }
        
        info!("Loaded {} accounts", accounts.len());
        Ok(())
    }
    
    /// Load transactions from storage
    async fn load_transactions(&self) -> LedgerResult<()> {
        let transaction_keys = self.storage.list(TRANSACTIONS_PATH).await?;
        let mut transactions = self.transactions.write().await;
        let mut transactions_by_account = self.transactions_by_account.write().await;
        
        for key in transaction_keys {
            match self.storage.get_json::<Transaction>(&key).await {
                Ok(transaction) => {
                    // Add to accounts cache
                    transactions.insert(transaction.id.clone(), transaction.clone());
                    
                    // Add to transactions by account
                    // From account
                    let tx_set = transactions_by_account
                        .entry(transaction.from_account.clone())
                        .or_insert_with(HashSet::new);
                    tx_set.insert(transaction.id.clone());
                    
                    // To account (if applicable)
                    if let Some(to_account) = &transaction.to_account {
                        let tx_set = transactions_by_account
                            .entry(to_account.clone())
                            .or_insert_with(HashSet::new);
                        tx_set.insert(transaction.id.clone());
                    }
                },
                Err(e) => {
                    error!("Failed to load transaction {}: {}", key, e);
                }
            }
        }
        
        info!("Loaded {} transactions", transactions.len());
        Ok(())
    }
    
    /// Save an account to storage
    async fn save_account(&self, account: &Account) -> LedgerResult<()> {
        // Save to storage
        let path = format!("{}/{}", ACCOUNTS_PATH, account.id);
        self.storage.put_json(&path, account).await?;
        
        // Update cache
        let mut accounts = self.accounts.write().await;
        accounts.insert(account.id.clone(), account.clone());
        
        // Update owner index
        let mut accounts_by_owner = self.accounts_by_owner.write().await;
        let owner_id = account.owner_id.as_str().to_string();
        let account_set = accounts_by_owner
            .entry(owner_id)
            .or_insert_with(HashSet::new);
        account_set.insert(account.id.clone());
        
        Ok(())
    }
    
    /// Save a transaction to storage
    async fn save_transaction(&self, transaction: &Transaction) -> LedgerResult<()> {
        // Save to storage
        let path = format!("{}/{}", TRANSACTIONS_PATH, transaction.id);
        self.storage.put_json(&path, transaction).await?;
        
        // Update cache
        let mut transactions = self.transactions.write().await;
        transactions.insert(transaction.id.clone(), transaction.clone());
        
        // Update account transaction index
        let mut transactions_by_account = self.transactions_by_account.write().await;
        
        // From account
        let tx_set = transactions_by_account
            .entry(transaction.from_account.clone())
            .or_insert_with(HashSet::new);
        tx_set.insert(transaction.id.clone());
        
        // To account (if applicable)
        if let Some(to_account) = &transaction.to_account {
            let tx_set = transactions_by_account
                .entry(to_account.clone())
                .or_insert_with(HashSet::new);
            tx_set.insert(transaction.id.clone());
        }
        
        Ok(())
    }
    
    /// Verify if the current user owns an account
    async fn verify_account_ownership(&self, account_id: &str) -> LedgerResult<bool> {
        // Get the current identity
        let identity = self.identity_provider.get_identity().await?;
        
        // Get the account
        let accounts = self.accounts.read().await;
        let account = accounts.get(account_id).ok_or_else(|| {
            LedgerError::AccountNotFound(account_id.to_string())
        })?;
        
        // Check if the identity is the owner
        Ok(account.owner_id == identity.id)
    }
    
    /// Process a transaction to update account balances
    async fn process_transaction(&self, transaction_id: &str) -> LedgerResult<Transaction> {
        // Use the transaction processor to process the transaction
        let transaction = self.transaction_processor
            .process_transaction(transaction_id, &*self.accounts, &*self.transactions)
            .await?;
        
        // Save the updated transaction
        self.save_transaction(&transaction).await?;
        
        // If transaction is confirmed, update account balances
        if transaction.status == TransactionStatus::Confirmed {
            match transaction.transaction_type {
                TransactionType::Transfer => {
                    self.update_balances_for_transfer(&transaction).await?;
                },
                TransactionType::Clearing => {
                    self.update_balances_for_clearing(&transaction).await?;
                },
                TransactionType::Issuance => {
                    self.update_balances_for_issuance(&transaction).await?;
                },
                _ => {
                    // Other transaction types might not affect balances
                }
            }
        }
        
        Ok(transaction)
    }
    
    /// Update account balances for a transfer transaction
    async fn update_balances_for_transfer(&self, transaction: &Transaction) -> LedgerResult<()> {
        // Get accounts
        let from_account_id = &transaction.from_account;
        let to_account_id = transaction.to_account.as_ref().ok_or_else(|| {
            LedgerError::InvalidTransaction("Transfer transaction must have a recipient".to_string())
        })?;
        
        // Get and update the accounts
        let mut accounts = self.accounts.write().await;
        
        // Get from account
        let from_account = accounts.get_mut(from_account_id).ok_or_else(|| {
            LedgerError::AccountNotFound(from_account_id.clone())
        })?;
        
        // Apply debit to from_account
        from_account.apply_debit(transaction.amount, &transaction.id)?;
        
        // Get to account
        let to_account = accounts.get_mut(to_account_id).ok_or_else(|| {
            LedgerError::AccountNotFound(to_account_id.clone())
        })?;
        
        // Apply credit to to_account
        to_account.apply_credit(transaction.amount, &transaction.id);
        
        // Save the updated accounts
        let from_account_clone = from_account.clone();
        let to_account_clone = to_account.clone();
        
        // Release the write lock before saving
        drop(accounts);
        
        // Save the accounts
        self.save_account(&from_account_clone).await?;
        self.save_account(&to_account_clone).await?;
        
        Ok(())
    }
    
    /// Update account balances for a clearing transaction
    async fn update_balances_for_clearing(&self, transaction: &Transaction) -> LedgerResult<()> {
        // Clearing is similar to transfer, but the amount is already calculated as the mutual debt
        self.update_balances_for_transfer(transaction).await
    }
    
    /// Update account balances for an issuance transaction
    async fn update_balances_for_issuance(&self, transaction: &Transaction) -> LedgerResult<()> {
        // Issuance only credits the recipient account
        let to_account_id = transaction.to_account.as_ref().ok_or_else(|| {
            LedgerError::InvalidTransaction("Issuance transaction must have a recipient".to_string())
        })?;
        
        // Get and update the accounts
        let mut accounts = self.accounts.write().await;
        
        // Get to account
        let to_account = accounts.get_mut(to_account_id).ok_or_else(|| {
            LedgerError::AccountNotFound(to_account_id.clone())
        })?;
        
        // Apply credit to to_account
        to_account.apply_credit(transaction.amount, &transaction.id);
        
        // Save the updated account
        let to_account_clone = to_account.clone();
        
        // Release the write lock before saving
        drop(accounts);
        
        // Save the account
        self.save_account(&to_account_clone).await?;
        
        Ok(())
    }
    
    /// Calculate and clear mutual debt between two accounts
    async fn calculate_mutual_debt(&self, account1_id: &str, account2_id: &str) -> LedgerResult<Option<f64>> {
        // Get the accounts
        let accounts = self.accounts.read().await;
        
        let account1 = accounts.get(account1_id).ok_or_else(|| {
            LedgerError::AccountNotFound(account1_id.to_string())
        })?;
        
        let account2 = accounts.get(account2_id).ok_or_else(|| {
            LedgerError::AccountNotFound(account2_id.to_string())
        })?;
        
        // Check if they use the same currency
        if account1.currency != account2.currency {
            return Err(LedgerError::InvalidTransaction(
                format!("Cannot clear debt between accounts with different currencies: {} and {}", 
                        account1.currency, account2.currency)
            ));
        }
        
        // Get transactions between these accounts
        let transactions_to_check = self.get_transactions_between_accounts(account1_id, account2_id).await?;
        
        // Calculate the net balance between the accounts
        let mut net_balance = 0.0;
        
        for tx in &transactions_to_check {
            if tx.status != TransactionStatus::Confirmed {
                continue;
            }
            
            if tx.from_account == *account1_id && tx.to_account.as_deref() == Some(account2_id) {
                net_balance -= tx.amount;
            } else if tx.from_account == *account2_id && tx.to_account.as_deref() == Some(account1_id) {
                net_balance += tx.amount;
            }
        }
        
        // If there's no debt to clear, return None
        if net_balance.abs() < 0.000001 {
            return Ok(None);
        }
        
        Ok(Some(net_balance))
    }
    
    /// Get transactions between two accounts
    async fn get_transactions_between_accounts(&self, account1_id: &str, account2_id: &str) -> LedgerResult<Vec<Transaction>> {
        let transactions = self.transactions.read().await;
        let transactions_by_account = self.transactions_by_account.read().await;
        
        // Get transaction IDs for account1
        let account1_tx_ids = match transactions_by_account.get(account1_id) {
            Some(ids) => ids,
            None => return Ok(Vec::new()),
        };
        
        // Get transaction IDs for account2
        let account2_tx_ids = match transactions_by_account.get(account2_id) {
            Some(ids) => ids,
            None => return Ok(Vec::new()),
        };
        
        // Find transactions that involve both accounts
        let mut result = Vec::new();
        
        for tx_id in account1_tx_ids {
            if !account2_tx_ids.contains(tx_id) {
                continue;
            }
            
            if let Some(tx) = transactions.get(tx_id) {
                // Check if it's a transaction between these two accounts
                if (tx.from_account == *account1_id && tx.to_account.as_deref() == Some(account2_id)) ||
                   (tx.from_account == *account2_id && tx.to_account.as_deref() == Some(account1_id)) {
                    result.push(tx.clone());
                }
            }
        }
        
        Ok(result)
    }
}

#[async_trait]
impl Ledger for MutualCreditLedger {
    async fn get_config(&self) -> LedgerResult<LedgerConfig> {
        let config = self.config.read().await;
        Ok(config.clone())
    }
    
    async fn set_config(&self, config: LedgerConfig) -> LedgerResult<()> {
        // Save to storage first
        self.storage.put_json(CONFIG_PATH, &config).await?;
        
        // Update local cache
        {
            let mut local_config = self.config.write().await;
            *local_config = config;
        }
        
        Ok(())
    }
    
    async fn create_account(
        &self,
        name: String,
        currency: Option<String>,
        credit_limit: Option<f64>,
        metadata: HashMap<String, String>,
    ) -> LedgerResult<Account> {
        // Use the account manager to create the account
        let account = self.account_manager.create_account(name, currency, credit_limit, metadata).await?;
        
        // Save the account
        self.save_account(&account).await?;
        
        Ok(account)
    }
    
    async fn get_account(&self, id: &str) -> LedgerResult<Option<Account>> {
        let accounts = self.accounts.read().await;
        Ok(accounts.get(id).cloned())
    }
    
    async fn get_accounts_by_owner(&self, owner_id: &NodeId) -> LedgerResult<Vec<Account>> {
        let accounts_by_owner = self.accounts_by_owner.read().await;
        let accounts = self.accounts.read().await;
        
        let owner_id_str = owner_id.as_str().to_string();
        let account_ids = match accounts_by_owner.get(&owner_id_str) {
            Some(ids) => ids,
            None => return Ok(Vec::new()),
        };
        
        let mut result = Vec::new();
        for id in account_ids {
            if let Some(account) = accounts.get(id) {
                result.push(account.clone());
            }
        }
        
        Ok(result)
    }
    
    async fn update_account_metadata(
        &self,
        account_id: &str,
        metadata: HashMap<String, String>,
    ) -> LedgerResult<Account> {
        // Verify ownership
        if !self.verify_account_ownership(account_id).await? {
            return Err(LedgerError::PermissionDenied(
                format!("You do not own account {}", account_id)
            ));
        }
        
        // Get the account
        let mut accounts = self.accounts.write().await;
        let account = accounts.get_mut(account_id).ok_or_else(|| {
            LedgerError::AccountNotFound(account_id.to_string())
        })?;
        
        // Update metadata
        account.metadata = metadata;
        account.updated_at = timestamp_secs();
        
        // Make a clone for saving
        let account_clone = account.clone();
        
        // Release the write lock before saving
        drop(accounts);
        
        // Save the account
        self.save_account(&account_clone).await?;
        
        Ok(account_clone)
    }
    
    async fn update_credit_limit(
        &self,
        account_id: &str,
        new_limit: f64,
    ) -> LedgerResult<Account> {
        // Verify ownership
        if !self.verify_account_ownership(account_id).await? {
            return Err(LedgerError::PermissionDenied(
                format!("You do not own account {}", account_id)
            ));
        }
        
        // Get the account
        let mut accounts = self.accounts.write().await;
        let account = accounts.get_mut(account_id).ok_or_else(|| {
            LedgerError::AccountNotFound(account_id.to_string())
        })?;
        
        // Update credit limit
        account.update_credit_limit(new_limit)?;
        
        // Make a clone for saving
        let account_clone = account.clone();
        
        // Release the write lock before saving
        drop(accounts);
        
        // Save the account
        self.save_account(&account_clone).await?;
        
        // Create a credit limit adjustment transaction
        let mut metadata = HashMap::new();
        metadata.insert("previous_limit".to_string(), account_clone.credit_limit.to_string());
        metadata.insert("new_limit".to_string(), new_limit.to_string());
        
        self.create_transaction(
            TransactionType::CreditLimitAdjustment,
            account_id,
            None,
            0.0, // No actual amount transferred
            Some(account_clone.currency.clone()),
            format!("Credit limit adjusted to {}", new_limit),
            metadata,
            Vec::new(),
        ).await?;
        
        Ok(account_clone)
    }
    
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
    ) -> LedgerResult<Transaction> {
        // Verify ownership if creating a transfer
        if transaction_type == TransactionType::Transfer && !self.verify_account_ownership(from_account).await? {
            return Err(LedgerError::PermissionDenied(
                format!("You do not own account {}", from_account)
            ));
        }
        
        // Get the accounts for validation
        let accounts = self.accounts.read().await;
        
        // Get from account
        let from_account_obj = accounts.get(from_account).ok_or_else(|| {
            LedgerError::AccountNotFound(from_account.to_string())
        })?;
        
        // Determine currency
        let currency = currency.unwrap_or_else(|| from_account_obj.currency.clone());
        
        // Validate to_account if provided
        let to_account_obj = if let Some(to_id) = to_account {
            let to_account = accounts.get(to_id).ok_or_else(|| {
                LedgerError::AccountNotFound(to_id.to_string())
            })?;
            
            // Check currencies match
            if to_account.currency != currency {
                return Err(LedgerError::InvalidTransaction(
                    format!("Currency mismatch: from account uses {}, to account uses {}", 
                            currency, to_account.currency)
                ));
            }
            
            Some(to_account)
        } else {
            None
        };
        
        // Get config for limits
        let config = self.config.read().await;
        
        // Check transaction amount limit for transfers
        if transaction_type == TransactionType::Transfer && amount > config.max_transaction_amount {
            return Err(LedgerError::InvalidTransaction(
                format!("Transaction amount {} exceeds maximum allowed ({})", 
                        amount, config.max_transaction_amount)
            ));
        }
        
        // Check if transfer is within credit limit
        if transaction_type == TransactionType::Transfer && !from_account_obj.can_debit(amount) {
            return Err(LedgerError::CreditLimitExceeded(
                format!("Transaction would exceed credit limit of {}", from_account_obj.credit_limit)
            ));
        }
        
        // Create the transaction
        let mut transaction = Transaction::new(
            transaction_type,
            from_account.to_string(),
            to_account.map(|s| s.to_string()),
            amount,
            currency,
            description,
            metadata,
            references,
        );
        
        // Sign the transaction
        let bytes_to_sign = transaction.bytes_to_sign();
        let signature = self.identity_provider.sign(&bytes_to_sign).await?;
        transaction.signature = signature;
        
        // Save the transaction
        self.save_transaction(&transaction).await?;
        
        // If counter-signatures are not required, process it immediately for certain types
        if !config.require_counter_signatures && 
           transaction_type != TransactionType::Transfer {
            // Process the transaction
            transaction = self.process_transaction(&transaction.id).await?;
        }
        
        Ok(transaction)
    }
    
    async fn get_transaction(&self, id: &str) -> LedgerResult<Option<Transaction>> {
        let transactions = self.transactions.read().await;
        Ok(transactions.get(id).cloned())
    }
    
    async fn get_transactions_by_account(
        &self,
        account_id: &str,
    ) -> LedgerResult<Vec<Transaction>> {
        let transactions_by_account = self.transactions_by_account.read().await;
        let transactions = self.transactions.read().await;
        
        // Get transaction IDs for this account
        let transaction_ids = match transactions_by_account.get(account_id) {
            Some(ids) => ids,
            None => return Ok(Vec::new()),
        };
        
        // Collect the transactions
        let mut result = Vec::new();
        for id in transaction_ids {
            if let Some(transaction) = transactions.get(id) {
                result.push(transaction.clone());
            }
        }
        
        // Sort by creation time, newest first
        result.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        
        Ok(result)
    }
    
    async fn counter_sign_transaction(&self, id: &str) -> LedgerResult<Transaction> {
        // Get the transaction
        let transactions = self.transactions.read().await;
        let mut transaction = transactions.get(id).cloned().ok_or_else(|| {
            LedgerError::TransactionNotFound(id.to_string())
        })?;
        
        // Release the read lock
        drop(transactions);
        
        // Check if the transaction needs a counter-signature
        if transaction.counter_signature.is_some() {
            return Err(LedgerError::InvalidTransaction(
                "Transaction already has a counter-signature".to_string()
            ));
        }
        
        // Get recipient account
        let to_account_id = transaction.to_account.as_ref().ok_or_else(|| {
            LedgerError::InvalidTransaction("Transaction has no recipient account".to_string())
        })?;
        
        // Verify the current user owns the recipient account
        if !self.verify_account_ownership(to_account_id).await? {
            return Err(LedgerError::PermissionDenied(
                format!("You do not own account {}", to_account_id)
            ));
        }
        
        // Sign the transaction
        let bytes_to_sign = transaction.bytes_to_sign();
        let signature = self.identity_provider.sign(&bytes_to_sign).await?;
        transaction.counter_signature = Some(signature);
        
        // Save the transaction
        self.save_transaction(&transaction).await?;
        
        // Process the transaction
        transaction = self.process_transaction(&transaction.id).await?;
        
        Ok(transaction)
    }
    
    async fn confirm_transaction(&self, id: &str) -> LedgerResult<Transaction> {
        // Get the transaction
        let transactions = self.transactions.read().await;
        let transaction = transactions.get(id).cloned().ok_or_else(|| {
            LedgerError::TransactionNotFound(id.to_string())
        })?;
        
        // Release the read lock
        drop(transactions);
        
        // Check if the transaction is already confirmed
        if transaction.status != TransactionStatus::Pending {
            return Err(LedgerError::InvalidTransaction(
                format!("Transaction is not pending, current status: {:?}", transaction.status)
            ));
        }
        
        // Get configuration for counter-signature requirement
        let config = self.config.read().await;
        
        // Check if we need a counter-signature for this transaction
        if config.require_counter_signatures && 
           transaction.transaction_type == TransactionType::Transfer && 
           transaction.counter_signature.is_none() {
            return Err(LedgerError::InvalidTransaction(
                "Transfer transaction requires counter-signature".to_string()
            ));
        }
        
        // Process the transaction
        let updated_transaction = self.process_transaction(&transaction.id).await?;
        
        Ok(updated_transaction)
    }
    
    async fn cancel_transaction(&self, id: &str) -> LedgerResult<Transaction> {
        // Get the transaction
        let transactions = self.transactions.read().await;
        let mut transaction = transactions.get(id).cloned().ok_or_else(|| {
            LedgerError::TransactionNotFound(id.to_string())
        })?;
        
        // Release the read lock
        drop(transactions);
        
        // Check if the transaction can be cancelled
        if transaction.status != TransactionStatus::Pending {
            return Err(LedgerError::InvalidTransaction(
                format!("Only pending transactions can be cancelled, current status: {:?}", transaction.status)
            ));
        }
        
        // Verify the current user owns the source account
        if !self.verify_account_ownership(&transaction.from_account).await? {
            return Err(LedgerError::PermissionDenied(
                format!("You do not own account {}", transaction.from_account)
            ));
        }
        
        // Mark as cancelled
        transaction.status = TransactionStatus::Cancelled;
        transaction.confirmed_at = Some(timestamp_secs());
        
        // Save the transaction
        self.save_transaction(&transaction).await?;
        
        Ok(transaction)
    }
    
    async fn get_balance(&self, account_id: &str) -> LedgerResult<f64> {
        let accounts = self.accounts.read().await;
        let account = accounts.get(account_id).ok_or_else(|| {
            LedgerError::AccountNotFound(account_id.to_string())
        })?;
        
        Ok(account.balance)
    }
    
    async fn clear_mutual_debt(
        &self,
        account1_id: &str,
        account2_id: &str,
    ) -> LedgerResult<Option<Transaction>> {
        // Verify ownership of at least one of the accounts
        if !self.verify_account_ownership(account1_id).await? && 
           !self.verify_account_ownership(account2_id).await? {
            return Err(LedgerError::PermissionDenied(
                "You must own at least one of the accounts".to_string()
            ));
        }
        
        // Calculate the debt to clear
        let debt_amount = match self.calculate_mutual_debt(account1_id, account2_id).await? {
            Some(amount) => amount,
            None => return Ok(None), // No debt to clear
        };
        
        // Determine which way to transfer
        let (from_id, to_id, amount) = if debt_amount > 0.0 {
            // Account 1 owes Account 2
            (account1_id, account2_id, debt_amount.abs())
        } else {
            // Account 2 owes Account 1
            (account2_id, account1_id, debt_amount.abs())
        };
        
        // Get account information for currency
        let accounts = self.accounts.read().await;
        let from_account = accounts.get(from_id).ok_or_else(|| {
            LedgerError::AccountNotFound(from_id.to_string())
        })?;
        
        // Release the read lock
        drop(accounts);
        
        // Create metadata
        let mut metadata = HashMap::new();
        metadata.insert("clearing_type".to_string(), "mutual_debt".to_string());
        metadata.insert("account1".to_string(), account1_id.to_string());
        metadata.insert("account2".to_string(), account2_id.to_string());
        
        // Create a clearing transaction
        let transaction = self.create_transaction(
            TransactionType::Clearing,
            from_id,
            Some(to_id),
            amount,
            Some(from_account.currency.clone()),
            format!("Clearing mutual debt between accounts"),
            metadata,
            Vec::new(),
        ).await?;
        
        // Confirm the transaction without requiring counter-signature
        let transaction = self.confirm_transaction(&transaction.id).await?;
        
        Ok(Some(transaction))
    }
} 