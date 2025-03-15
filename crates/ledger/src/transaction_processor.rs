//! Transaction processor for ledger transactions
//!
//! This module provides functionality for processing and validating
//! transactions in the mutual credit ledger.

use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

use icn_core::{
    storage::{Storage, JsonStorage},
    crypto::{NodeId, Signature, verify_signature},
    utils::timestamp_secs,
};

use icn_identity::{
    IdentityProvider, IdentityResult, Identity,
};

use crate::{
    LedgerConfig, LedgerResult, LedgerError,
    Account, Transaction, TransactionStatus, TransactionType,
};

/// The transaction processor for handling ledger transactions
pub struct TransactionProcessor {
    /// Identity provider for signature verification
    identity_provider: Arc<dyn IdentityProvider>,
    /// Storage for transaction data
    storage: Arc<dyn Storage>,
    /// Configuration
    config: LedgerConfig,
}

impl TransactionProcessor {
    /// Create a new transaction processor
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
    
    /// Process a transaction and update its status
    pub async fn process_transaction(
        &self,
        transaction_id: &str,
        accounts: &HashMap<String, Account>,
        transactions: &HashMap<String, Transaction>,
    ) -> LedgerResult<Transaction> {
        // Get the transaction
        let mut transaction = transactions.get(transaction_id).cloned().ok_or_else(|| {
            LedgerError::TransactionNotFound(transaction_id.to_string())
        })?;
        
        // Check if it's already processed
        if transaction.status != TransactionStatus::Pending {
            return Ok(transaction);
        }
        
        // Validate the transaction
        if let Err(e) = self.validate_transaction(&transaction, accounts, transactions).await {
            // Mark as rejected
            transaction.status = TransactionStatus::Rejected;
            transaction.confirmed_at = Some(timestamp_secs());
            // Add rejection reason to metadata
            transaction.metadata.insert("rejection_reason".to_string(), e.to_string());
            return Ok(transaction);
        }
        
        // Mark as confirmed
        transaction.status = TransactionStatus::Confirmed;
        transaction.confirmed_at = Some(timestamp_secs());
        
        Ok(transaction)
    }
    
    /// Validate a transaction
    async fn validate_transaction(
        &self,
        transaction: &Transaction,
        accounts: &HashMap<String, Account>,
        transactions: &HashMap<String, Transaction>,
    ) -> LedgerResult<()> {
        // 1. Validate transaction signature
        self.validate_signature(transaction).await?;
        
        // 2. Validate counter-signature if required
        if self.config.require_counter_signatures && 
           transaction.transaction_type == TransactionType::Transfer {
            self.validate_counter_signature(transaction).await?;
        }
        
        // 3. Validate accounts exist
        let from_account = accounts.get(&transaction.from_account).ok_or_else(|| {
            LedgerError::AccountNotFound(transaction.from_account.clone())
        })?;
        
        let to_account = if let Some(to_account_id) = &transaction.to_account {
            Some(accounts.get(to_account_id).ok_or_else(|| {
                LedgerError::AccountNotFound(to_account_id.clone())
            })?)
        } else {
            None
        };
        
        // 4. Validate transaction type specifics
        match transaction.transaction_type {
            TransactionType::Transfer => {
                self.validate_transfer(transaction, from_account, to_account.unwrap())?;
            },
            TransactionType::Clearing => {
                self.validate_clearing(transaction, from_account, to_account.unwrap())?;
            },
            TransactionType::Issuance => {
                self.validate_issuance(transaction, to_account.unwrap())?;
            },
            TransactionType::AccountCreation => {
                // Account creation doesn't need special validation
            },
            TransactionType::AccountUpdate => {
                // Account update doesn't need special validation
            },
            TransactionType::CreditLimitAdjustment => {
                self.validate_credit_limit_adjustment(transaction, from_account)?;
            },
            TransactionType::Custom(ref custom_type) => {
                // Custom transaction types need application-specific validation
                debug!("Validating custom transaction type: {}", custom_type);
            },
        }
        
        // 5. Validate references if applicable
        if !transaction.references.is_empty() {
            self.validate_references(transaction, transactions)?;
        }
        
        Ok(())
    }
    
    /// Validate transaction signature
    async fn validate_signature(&self, transaction: &Transaction) -> LedgerResult<()> {
        // Get from account owner
        let from_account_owner = self.get_account_owner(&transaction.from_account).await?;
        
        // Get the bytes that were signed
        let bytes_to_sign = transaction.bytes_to_sign();
        
        // Verify the signature
        if !self.identity_provider.verify(
            &from_account_owner.id,
            &bytes_to_sign,
            &transaction.signature,
        ).await? {
            return Err(LedgerError::InvalidTransaction(
                "Invalid transaction signature".to_string()
            ));
        }
        
        Ok(())
    }
    
    /// Validate transaction counter-signature
    async fn validate_counter_signature(&self, transaction: &Transaction) -> LedgerResult<()> {
        // Check if counter-signature is required
        if !self.config.require_counter_signatures {
            return Ok(());
        }
        
        // Check if counter-signature is present
        let counter_signature = transaction.counter_signature.as_ref().ok_or_else(|| {
            LedgerError::InvalidTransaction("Missing required counter-signature".to_string())
        })?;
        
        // Get to account owner
        let to_account_id = transaction.to_account.as_ref().ok_or_else(|| {
            LedgerError::InvalidTransaction("Transaction has no recipient account".to_string())
        })?;
        
        let to_account_owner = self.get_account_owner(to_account_id).await?;
        
        // Get the bytes that were signed
        let bytes_to_sign = transaction.bytes_to_sign();
        
        // Verify the counter-signature
        if !self.identity_provider.verify(
            &to_account_owner.id,
            &bytes_to_sign,
            counter_signature,
        ).await? {
            return Err(LedgerError::InvalidTransaction(
                "Invalid transaction counter-signature".to_string()
            ));
        }
        
        Ok(())
    }
    
    /// Get account owner identity
    async fn get_account_owner(&self, account_id: &str) -> LedgerResult<Identity> {
        // In a real implementation, we would look up the account and then get its owner
        // For now, we'll just get the current identity
        let identity = self.identity_provider.get_identity().await?;
        Ok(identity)
    }
    
    /// Validate a transfer transaction
    fn validate_transfer(
        &self,
        transaction: &Transaction,
        from_account: &Account,
        to_account: &Account,
    ) -> LedgerResult<()> {
        // Check basic transfer requirements
        if !transaction.is_valid_transfer() {
            return Err(LedgerError::InvalidTransaction(
                "Transaction does not meet basic transfer requirements".to_string()
            ));
        }
        
        // Check currencies match
        if from_account.currency != to_account.currency {
            return Err(LedgerError::InvalidTransaction(
                format!("Currency mismatch: from account uses {}, to account uses {}", 
                        from_account.currency, to_account.currency)
            ));
        }
        
        // Check credit limit
        if !from_account.can_debit(transaction.amount) {
            return Err(LedgerError::CreditLimitExceeded(
                format!("Transfer would exceed credit limit of {}", from_account.credit_limit)
            ));
        }
        
        // Check transaction amount limit
        if transaction.amount > self.config.max_transaction_amount {
            return Err(LedgerError::InvalidTransaction(
                format!("Transaction amount {} exceeds maximum allowed ({})", 
                        transaction.amount, self.config.max_transaction_amount)
            ));
        }
        
        Ok(())
    }
    
    /// Validate a clearing transaction
    fn validate_clearing(
        &self,
        transaction: &Transaction,
        from_account: &Account,
        to_account: &Account,
    ) -> LedgerResult<()> {
        // Check currencies match
        if from_account.currency != to_account.currency {
            return Err(LedgerError::InvalidTransaction(
                format!("Currency mismatch: from account uses {}, to account uses {}", 
                        from_account.currency, to_account.currency)
            ));
        }
        
        // Check credit limit
        if !from_account.can_debit(transaction.amount) {
            return Err(LedgerError::CreditLimitExceeded(
                format!("Clearing would exceed credit limit of {}", from_account.credit_limit)
            ));
        }
        
        Ok(())
    }
    
    /// Validate an issuance transaction
    fn validate_issuance(
        &self,
        transaction: &Transaction,
        to_account: &Account,
    ) -> LedgerResult<()> {
        // Check amount is positive
        if transaction.amount <= 0.0 {
            return Err(LedgerError::InvalidTransaction(
                "Issuance amount must be positive".to_string()
            ));
        }
        
        // Check currency matches
        if transaction.currency != to_account.currency {
            return Err(LedgerError::InvalidTransaction(
                format!("Currency mismatch: transaction uses {}, account uses {}", 
                        transaction.currency, to_account.currency)
            ));
        }
        
        // In a real system, check if the sender is authorized to issue currency
        // This is a placeholder for that check
        
        Ok(())
    }
    
    /// Validate a credit limit adjustment transaction
    fn validate_credit_limit_adjustment(
        &self,
        transaction: &Transaction,
        account: &Account,
    ) -> LedgerResult<()> {
        // Get the new limit from metadata
        let new_limit = transaction.metadata.get("new_limit")
            .ok_or_else(|| {
                LedgerError::InvalidTransaction("Missing new_limit in metadata".to_string())
            })?
            .parse::<f64>()
            .map_err(|_| {
                LedgerError::InvalidTransaction("Invalid new_limit format".to_string())
            })?;
        
        // Check if new limit is valid
        if new_limit < 0.0 {
            return Err(LedgerError::InvalidTransaction(
                "Credit limit cannot be negative".to_string()
            ));
        }
        
        // Check if the new limit would put the account over limit
        if -account.balance > new_limit {
            return Err(LedgerError::CreditLimitExceeded(
                format!("Current balance ({}) exceeds new credit limit ({})", 
                        account.balance, new_limit)
            ));
        }
        
        Ok(())
    }
    
    /// Validate transaction references
    fn validate_references(
        &self,
        transaction: &Transaction,
        transactions: &HashMap<String, Transaction>,
    ) -> LedgerResult<()> {
        // Check that all referenced transactions exist and are confirmed
        for ref_id in &transaction.references {
            let ref_tx = transactions.get(ref_id).ok_or_else(|| {
                LedgerError::TransactionNotFound(ref_id.clone())
            })?;
            
            if ref_tx.status != TransactionStatus::Confirmed {
                return Err(LedgerError::InvalidTransaction(
                    format!("Referenced transaction {} is not confirmed", ref_id)
                ));
            }
        }
        
        Ok(())
    }
    
    /// Calculate transaction fee (if applicable)
    pub fn calculate_fee(&self, transaction: &Transaction) -> f64 {
        // In this simple implementation, we don't charge fees
        // This is a placeholder for a real fee calculation
        0.0
    }
    
    /// Check if a transaction would trigger an alert
    pub fn check_transaction_alerts(&self, transaction: &Transaction) -> Vec<String> {
        let mut alerts = Vec::new();
        
        // Check for large transfers
        if transaction.transaction_type == TransactionType::Transfer && 
           transaction.amount > self.config.max_transaction_amount * 0.8 {
            alerts.push(format!("Large transfer: {:.2} {}", 
                                transaction.amount, transaction.currency));
        }
        
        // Check for unusual patterns
        // This would be more sophisticated in a real implementation
        
        alerts
    }
    
    /// Generate a transaction receipt
    pub fn generate_receipt(&self, transaction: &Transaction) -> TransactionReceipt {
        TransactionReceipt {
            transaction_id: transaction.id.clone(),
            transaction_type: transaction.transaction_type.clone(),
            from_account: transaction.from_account.clone(),
            to_account: transaction.to_account.clone(),
            amount: transaction.amount,
            currency: transaction.currency.clone(),
            fee: self.calculate_fee(transaction),
            timestamp: transaction.confirmed_at.unwrap_or(transaction.created_at),
            status: transaction.status,
        }
    }
}

/// A receipt for a processed transaction
#[derive(Debug, Clone)]
pub struct TransactionReceipt {
    /// The transaction ID
    pub transaction_id: String,
    /// The type of transaction
    pub transaction_type: TransactionType,
    /// The source account
    pub from_account: String,
    /// The destination account (if applicable)
    pub to_account: Option<String>,
    /// The amount of the transaction
    pub amount: f64,
    /// The currency of the transaction
    pub currency: String,
    /// The fee charged (if any)
    pub fee: f64,
    /// When the transaction was processed
    pub timestamp: u64,
    /// The status of the transaction
    pub status: TransactionStatus,
} 