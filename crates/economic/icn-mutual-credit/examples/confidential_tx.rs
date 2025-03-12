//! Example demonstrating confidential transactions in the mutual credit system

use icn_mutual_credit::{
    Account, AccountStatus, Amount, CreditGraph, CreditLine, CreditTerms,
    TransactionProcessor, DID, Transaction, TransactionType,
};

use std::sync::Arc;
use tokio::sync::Mutex;
use rust_decimal::Decimal;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a sample credit graph for testing
    let mut graph = CreditGraph::new();
    
    // Create accounts for Alice and Bob
    let alice_did = DID::new("did:icn:alice");
    let bob_did = DID::new("did:icn:bob");
    
    println!("🔑 Creating accounts for Alice and Bob");
    
    let alice = Account::new(alice_did.clone(), "Alice".to_string());
    let bob = Account::new(bob_did.clone(), "Bob".to_string());
    
    // Add accounts to the graph
    graph.add_account(alice).await?;
    graph.add_account(bob).await?;
    
    println!("✅ Accounts created successfully");
    
    // Create bidirectional credit lines between Alice and Bob
    println!("🔄 Establishing credit lines");
    
    // Alice can extend up to 1000 credit to Bob
    let alice_to_bob = CreditLine::new(
        alice_did.clone(),
        bob_did.clone(),
        Amount::new(1000),
        CreditTerms::new(),
    );
    
    // Bob can extend up to 800 credit to Alice
    let bob_to_alice = CreditLine::new(
        bob_did.clone(),
        alice_did.clone(),
        Amount::new(800),
        CreditTerms::new(),
    );
    
    graph.add_credit_line(alice_to_bob).await?;
    graph.add_credit_line(bob_to_alice).await?;
    
    println!("✅ Credit lines established");
    
    // Create a transaction processor
    let graph_arc = Arc::new(Mutex::new(graph));
    let mut processor = TransactionProcessor::new(graph_arc.clone(), None);
    
    // Create a standard transaction from Alice to Bob (for comparison)
    println!("\n🔄 Creating a standard transaction from Alice to Bob of 200 units");
    
    let standard_tx = Transaction::new(
        "tx-1".to_string(),
        alice_did.clone(),
        bob_did.clone(),
        Amount::new(200),
        TransactionType::DirectTransfer,
        Some("Standard payment from Alice to Bob".to_string()),
    );
    
    // Submit the transaction
    processor.submit_transaction(standard_tx.clone()).await?;
    
    // Process all pending transactions
    let results = processor.process_pending_transactions().await;
    for result in results {
        if let Err(e) = result {
            println!("Error processing transaction: {:?}", e);
        }
    }
    
    // Check balances after standard transaction
    let graph_lock = graph_arc.lock().await;
    let alice_balance = graph_lock.get_account_balance(&alice_did).await?;
    let bob_balance = graph_lock.get_account_balance(&bob_did).await?;
    drop(graph_lock);
    
    println!("📊 Balances after standard transaction:");
    println!("   Alice: {}", alice_balance);
    println!("   Bob:   {}", bob_balance);
    
    // Now create a confidential transaction from Bob to Alice
    println!("\n🔒 Creating a confidential transaction from Bob to Alice of 150 units");
    
    let confidential_tx_id = processor.create_confidential_transaction(
        &bob_did,
        &alice_did,
        Amount::new(150),
        Some("Confidential payment from Bob to Alice".to_string()),
    ).await?;
    
    // Process all pending transactions
    let results = processor.process_pending_transactions().await;
    for result in results {
        if let Err(e) = result {
            println!("Error processing transaction: {:?}", e);
        }
    }
    
    // Check balances after confidential transaction
    let graph_lock = graph_arc.lock().await;
    let alice_balance = graph_lock.get_account_balance(&alice_did).await?;
    let bob_balance = graph_lock.get_account_balance(&bob_did).await?;
    drop(graph_lock);
    
    println!("📊 Balances after confidential transaction:");
    println!("   Alice: {}", alice_balance);
    println!("   Bob:   {}", bob_balance);
    
    // Get transaction history
    let history = processor.get_transaction_history();
    
    println!("\n📜 Transaction History:");
    for (i, tx_result) in history.iter().enumerate() {
        println!("Transaction #{}", i + 1);
        println!("  ID:      {}", tx_result.transaction.id);
        println!("  From:    {}", tx_result.transaction.from);
        println!("  To:      {}", tx_result.transaction.to);
        println!("  Amount:  {}", tx_result.transaction.amount);
        println!("  Status:  {:?}", tx_result.transaction.status);
        println!("  Description: {:?}", tx_result.transaction.description);
        
        // For confidential transactions, we would normally not be able to see the amount
        // But in test mode, we can reveal it
        if tx_result.transaction.id == confidential_tx_id {
            println!("  🔓 This is a confidential transaction");
            
            #[cfg(test)] // Only works in test mode
            {
                let revealed_amount = processor.reveal_confidential_amount(&confidential_tx_id)?;
                println!("  🔍 Revealed Amount: {}", revealed_amount);
            }
        }
        
        println!();
    }
    
    println!("✅ Example completed successfully");
    
    Ok(())
} 