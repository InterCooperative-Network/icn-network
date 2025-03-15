//! Tests for the transaction processor
//!
//! These tests verify the core functionality of the transaction processor
//! in validating and processing transactions.

use std::collections::HashMap;
use std::sync::Arc;

use icn_core::{
    storage::{StorageError, MockStorage},
    crypto::{NodeId, Signature},
    utils::timestamp_secs,
};

use icn_identity::{
    Identity, IdentityResult, MockIdentityProvider,
};

use icn_ledger::{
    Transaction, Account, TransactionType, TransactionStatus, LedgerConfig,
    LedgerError, LedgerResult, transaction_processor::TransactionProcessor,
};

#[tokio::test]
async fn test_process_valid_transfer() {
    // Set up test environment
    let (identity_provider, storage, processor, mut accounts, mut transactions) = setup_test_env().await;
    
    // Create a valid transfer transaction
    let from_account = accounts.values().next().unwrap().clone();
    let to_account = accounts.values().nth(1).unwrap().clone();
    
    let transaction = create_test_transfer(
        "transfer1",
        from_account.id.clone(),
        to_account.id.clone(),
        50.0,
        &from_account.currency,
    );
    
    transactions.insert(transaction.id.clone(), transaction.clone());
    
    // Process the transaction
    let result = processor.process_transaction(
        &transaction.id, 
        &accounts, 
        &transactions,
    ).await;
    
    // Verify the transaction was confirmed
    assert!(result.is_ok());
    let processed = result.unwrap();
    assert_eq!(processed.status, TransactionStatus::Confirmed);
    assert!(processed.confirmed_at.is_some());
}

#[tokio::test]
async fn test_process_invalid_transfer_wrong_currency() {
    // Set up test environment
    let (identity_provider, storage, processor, mut accounts, mut transactions) = setup_test_env().await;
    
    // Create two accounts with different currencies
    let from_account = accounts.values().next().unwrap().clone();
    let mut to_account = accounts.values().nth(1).unwrap().clone();
    to_account.currency = "EUR".to_string();
    accounts.insert(to_account.id.clone(), to_account.clone());
    
    // Create a transfer transaction with mismatched currencies
    let transaction = create_test_transfer(
        "transfer2",
        from_account.id.clone(),
        to_account.id.clone(),
        50.0,
        &from_account.currency,
    );
    
    transactions.insert(transaction.id.clone(), transaction.clone());
    
    // Process the transaction
    let result = processor.process_transaction(
        &transaction.id, 
        &accounts, 
        &transactions,
    ).await;
    
    // Verify the transaction was rejected
    assert!(result.is_ok());
    let processed = result.unwrap();
    assert_eq!(processed.status, TransactionStatus::Rejected);
    assert!(processed.confirmed_at.is_some());
    assert!(processed.metadata.contains_key("rejection_reason"));
    assert!(processed.metadata.get("rejection_reason").unwrap().contains("Currency mismatch"));
}

#[tokio::test]
async fn test_process_invalid_transfer_exceeds_credit_limit() {
    // Set up test environment
    let (identity_provider, storage, processor, mut accounts, mut transactions) = setup_test_env().await;
    
    // Create a transfer transaction that exceeds credit limit
    let from_account = accounts.values().next().unwrap().clone();
    let to_account = accounts.values().nth(1).unwrap().clone();
    
    let transaction = create_test_transfer(
        "transfer3",
        from_account.id.clone(),
        to_account.id.clone(),
        1500.0, // Exceeds the 1000.0 max transaction amount
        &from_account.currency,
    );
    
    transactions.insert(transaction.id.clone(), transaction.clone());
    
    // Process the transaction
    let result = processor.process_transaction(
        &transaction.id, 
        &accounts, 
        &transactions,
    ).await;
    
    // Verify the transaction was rejected
    assert!(result.is_ok());
    let processed = result.unwrap();
    assert_eq!(processed.status, TransactionStatus::Rejected);
    assert!(processed.confirmed_at.is_some());
    assert!(processed.metadata.contains_key("rejection_reason"));
    assert!(processed.metadata.get("rejection_reason").unwrap().contains("exceeds maximum allowed"));
}

#[tokio::test]
async fn test_validate_credit_limit_adjustment() {
    // Set up test environment
    let (identity_provider, storage, processor, mut accounts, mut transactions) = setup_test_env().await;
    
    // Create a credit limit adjustment transaction
    let account = accounts.values().next().unwrap().clone();
    
    let mut metadata = HashMap::new();
    metadata.insert("new_limit".to_string(), "300.0".to_string());
    
    let transaction = Transaction {
        id: "adjust1".to_string(),
        transaction_type: TransactionType::CreditLimitAdjustment,
        from_account: account.id.clone(),
        to_account: None,
        amount: 0.0,
        currency: account.currency.clone(),
        description: "Adjust credit limit".to_string(),
        metadata,
        created_at: timestamp_secs(),
        confirmed_at: None,
        status: TransactionStatus::Pending,
        references: Vec::new(),
        signature: Vec::new(),
        counter_signature: None,
    };
    
    transactions.insert(transaction.id.clone(), transaction.clone());
    
    // Process the transaction
    let result = processor.process_transaction(
        &transaction.id, 
        &accounts, 
        &transactions,
    ).await;
    
    // Verify the transaction was confirmed
    assert!(result.is_ok());
    let processed = result.unwrap();
    assert_eq!(processed.status, TransactionStatus::Confirmed);
}

#[tokio::test]
async fn test_transaction_receipt_generation() {
    // Set up test environment
    let (identity_provider, storage, processor, mut accounts, mut transactions) = setup_test_env().await;
    
    // Create a confirmed transaction
    let from_account = accounts.values().next().unwrap().clone();
    let to_account = accounts.values().nth(1).unwrap().clone();
    
    let mut transaction = create_test_transfer(
        "receipt_test",
        from_account.id.clone(),
        to_account.id.clone(),
        50.0,
        &from_account.currency,
    );
    transaction.status = TransactionStatus::Confirmed;
    transaction.confirmed_at = Some(timestamp_secs());
    
    // Generate receipt
    let receipt = processor.generate_receipt(&transaction);
    
    // Verify receipt
    assert_eq!(receipt.transaction_id, transaction.id);
    assert_eq!(receipt.transaction_type, transaction.transaction_type);
    assert_eq!(receipt.from_account, transaction.from_account);
    assert_eq!(receipt.to_account, transaction.to_account);
    assert_eq!(receipt.amount, transaction.amount);
    assert_eq!(receipt.currency, transaction.currency);
    assert_eq!(receipt.status, transaction.status);
}

// Helper functions

/// Set up a test environment with mock dependencies
async fn setup_test_env() -> (
    Arc<MockIdentityProvider>,
    Arc<MockStorage>,
    TransactionProcessor,
    HashMap<String, Account>,
    HashMap<String, Transaction>,
) {
    // Create mocks
    let identity_provider = Arc::new(MockIdentityProvider::new());
    let storage = Arc::new(MockStorage::new());
    
    // Create config
    let config = LedgerConfig {
        default_credit_limit: 100.0,
        default_currency: "ICN".to_string(),
        supported_currencies: vec!["ICN".to_string()],
        max_transaction_amount: 1000.0,
        require_counter_signatures: false,
        custom_config: HashMap::new(),
    };
    
    // Create processor
    let processor = TransactionProcessor::new(
        identity_provider.clone(),
        storage.clone(),
        config,
    );
    
    // Create test accounts
    let mut accounts = HashMap::new();
    
    // Account 1
    let account1 = Account {
        id: "account1".to_string(),
        owner_id: "user1".to_string(),
        name: "Test Account 1".to_string(),
        currency: "ICN".to_string(),
        balance: 100.0,
        credit_limit: 200.0,
        transaction_history: Vec::new(),
        metadata: HashMap::new(),
        created_at: timestamp_secs(),
        updated_at: timestamp_secs(),
    };
    accounts.insert(account1.id.clone(), account1);
    
    // Account 2
    let account2 = Account {
        id: "account2".to_string(),
        owner_id: "user2".to_string(),
        name: "Test Account 2".to_string(),
        currency: "ICN".to_string(),
        balance: 50.0,
        credit_limit: 100.0,
        transaction_history: Vec::new(),
        metadata: HashMap::new(),
        created_at: timestamp_secs(),
        updated_at: timestamp_secs(),
    };
    accounts.insert(account2.id.clone(), account2);
    
    // Empty transactions
    let transactions = HashMap::new();
    
    (identity_provider, storage, processor, accounts, transactions)
}

/// Create a test transfer transaction
fn create_test_transfer(
    id: &str,
    from_account: String,
    to_account: String,
    amount: f64,
    currency: &str,
) -> Transaction {
    Transaction {
        id: id.to_string(),
        transaction_type: TransactionType::Transfer,
        from_account,
        to_account: Some(to_account),
        amount,
        currency: currency.to_string(),
        description: "Test transfer".to_string(),
        metadata: HashMap::new(),
        created_at: timestamp_secs(),
        confirmed_at: None,
        status: TransactionStatus::Pending,
        references: Vec::new(),
        signature: Vec::new(), // Mock signature
        counter_signature: None,
    }
} 