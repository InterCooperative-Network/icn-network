//! Basic example of using the mutual credit ledger
//!
//! This example demonstrates how to set up and use the mutual credit ledger
//! for tracking credits and debits between participants.

use std::collections::HashMap;
use std::sync::Arc;

use icn_core::storage::JsonStorage;
use icn_identity::{IdentityManager, IdentityProvider};
use icn_ledger::{
    Ledger, MutualCreditLedger, Account, Transaction, TransactionType, LedgerConfig,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set up temporary storage
    let storage_path = std::env::temp_dir().join("icn_ledger_example");
    println!("Using storage path: {:?}", storage_path);
    
    // Create storage if it doesn't exist
    std::fs::create_dir_all(&storage_path)?;
    
    // Initialize storage and identity provider
    let storage = Arc::new(JsonStorage::new(storage_path.to_str().unwrap()));
    let identity_provider = Arc::new(IdentityManager::new(storage.clone(), None).await?);
    
    // Create the mutual credit ledger
    let ledger_config = LedgerConfig::default();
    let ledger = MutualCreditLedger::new(
        identity_provider.clone(),
        storage.clone(),
        ledger_config,
    ).await?;
    
    // Create two accounts
    let alice_account = ledger.create_account(
        "Alice's Account".to_string(),
        None, // Use default currency
        Some(200.0), // Credit limit
        HashMap::new(),
    ).await?;
    
    let bob_account = ledger.create_account(
        "Bob's Account".to_string(),
        None, // Use default currency
        Some(200.0), // Credit limit
        HashMap::new(),
    ).await?;
    
    println!("Created accounts:");
    println!("  Alice: {}", alice_account.id);
    println!("  Bob: {}", bob_account.id);
    
    // Create a transaction from Alice to Bob
    let transfer = ledger.create_transaction(
        TransactionType::Transfer,
        &alice_account.id,
        Some(&bob_account.id),
        50.0,
        None, // Use account currency
        "Payment for services".to_string(),
        HashMap::new(),
        Vec::new(),
    ).await?;
    
    println!("\nCreated transfer transaction: {}", transfer.id);
    println!("  Status: {:?}", transfer.status);
    
    // Process the transaction
    let processed = ledger.confirm_transaction(&transfer.id).await?;
    println!("\nProcessed transaction: {}", processed.id);
    println!("  Status: {:?}", processed.status);
    
    // Check balances
    let alice_balance = ledger.get_balance(&alice_account.id).await?;
    let bob_balance = ledger.get_balance(&bob_account.id).await?;
    
    println!("\nBalances after transfer:");
    println!("  Alice: {} {}", alice_balance, alice_account.currency);
    println!("  Bob: {} {}", bob_balance, bob_account.currency);
    
    // Create a transaction from Bob to Alice
    let transfer_back = ledger.create_transaction(
        TransactionType::Transfer,
        &bob_account.id,
        Some(&alice_account.id),
        30.0,
        None, // Use account currency
        "Refund for overpayment".to_string(),
        HashMap::new(),
        Vec::new(),
    ).await?;
    
    // Process the transaction
    let processed_back = ledger.confirm_transaction(&transfer_back.id).await?;
    
    // Check balances again
    let alice_balance = ledger.get_balance(&alice_account.id).await?;
    let bob_balance = ledger.get_balance(&bob_account.id).await?;
    
    println!("\nBalances after second transfer:");
    println!("  Alice: {} {}", alice_balance, alice_account.currency);
    println!("  Bob: {} {}", bob_balance, bob_account.currency);
    
    // Try to clear mutual debt
    println!("\nAttempting to clear mutual debt...");
    let clearing = ledger.clear_mutual_debt(
        &alice_account.id,
        &bob_account.id,
    ).await?;
    
    if let Some(clearing_tx) = clearing {
        println!("Mutual debt cleared: {}", clearing_tx.id);
        println!("  Amount: {} {}", clearing_tx.amount, clearing_tx.currency);
        
        // Check final balances
        let alice_balance = ledger.get_balance(&alice_account.id).await?;
        let bob_balance = ledger.get_balance(&bob_account.id).await?;
        
        println!("\nFinal balances after clearing:");
        println!("  Alice: {} {}", alice_balance, alice_account.currency);
        println!("  Bob: {} {}", bob_balance, bob_account.currency);
    } else {
        println!("No mutual debt to clear.");
    }
    
    Ok(())
} 