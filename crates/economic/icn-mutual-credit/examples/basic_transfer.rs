use icn_mutual_credit::{
    Account, AccountStatus, Amount, CreditGraph, CreditLine, CreditLineId,
    CreditTerms, DID, Transaction, TransactionProcessor, TransactionResult, TransactionType,
};
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== ICN Mutual Credit - Basic Transfer Example ===\n");
    
    // Create DIDs for two cooperatives
    let coop1_did = DID::new("did:icn:coop:farming-collective");
    let coop2_did = DID::new("did:icn:coop:tech-support");
    
    println!("Created cooperatives:");
    println!("  - {}", coop1_did);
    println!("  - {}", coop2_did);
    
    // Create a credit graph
    let mut graph = CreditGraph::new();
    
    // Create and add accounts
    let coop1_account = Account::new(
        coop1_did.clone(),
        "Local Farming Collective".to_string(),
    );
    
    let coop2_account = Account::new(
        coop2_did.clone(),
        "Tech Support Cooperative".to_string(),
    );
    
    graph.add_account(coop1_account).await?;
    graph.add_account(coop2_account).await?;
    println!("\nAdded accounts to the credit graph");
    
    // Create and add credit lines
    let coop1_to_coop2 = CreditLine::new(
        coop1_did.clone(),
        coop2_did.clone(),
        Amount::new(100),  // Credit limit of 100 units
        CreditTerms::new(),
    );
    
    let coop2_to_coop1 = CreditLine::new(
        coop2_did.clone(),
        coop1_did.clone(),
        Amount::new(150),  // Credit limit of 150 units
        CreditTerms::new(),
    );
    
    graph.add_credit_line(coop1_to_coop2).await?;
    graph.add_credit_line(coop2_to_coop1).await?;
    println!("Created bilateral credit lines between cooperatives");
    
    // Wrap the graph in an Arc<Mutex> for the transaction processor
    let graph = Arc::new(Mutex::new(graph));
    
    // Create a transaction processor
    let mut processor = TransactionProcessor::new(Arc::clone(&graph), None);
    println!("\nCreated transaction processor");
    
    // Create a transaction: farming collective pays tech support for services
    let tx1 = Transaction::new(
        "tx-001".to_string(),
        coop1_did.clone(),
        coop2_did.clone(),
        Amount::new(30),
        TransactionType::DirectTransfer,
        Some("IT service and maintenance".to_string()),
    );
    
    println!("\nSubmitting transaction:");
    println!("  From: {} (Farming Collective)", coop1_did);
    println!("  To:   {} (Tech Support)", coop2_did);
    println!("  Amount: 30 units");
    println!("  Memo: IT service and maintenance");
    
    // Submit and process the transaction
    processor.submit_transaction(tx1).await?;
    let results = processor.process_pending_transactions().await;
    
    // Check and display results
    if let Some(Ok(result)) = results.first() {
        println!("\nTransaction completed successfully!");
        println!("  Transaction ID: {}", result.transaction.id);
        println!("  Status: Completed");
        println!("  Timestamp: {}", result.timestamp);
        
        // Display updated balances
        println!("\nUpdated balances:");
        for (did, balance) in &result.updated_balances {
            println!("  {}: {}", did, balance);
        }
    } else if let Some(Err(error)) = results.first() {
        println!("\nTransaction failed: {}", error);
    }
    
    // Now create a second transaction: tech support pays farming collective for food
    let tx2 = Transaction::new(
        "tx-002".to_string(),
        coop2_did.clone(),
        coop1_did.clone(),
        Amount::new(20),
        TransactionType::DirectTransfer,
        Some("Weekly food box delivery".to_string()),
    );
    
    println!("\nSubmitting second transaction:");
    println!("  From: {} (Tech Support)", coop2_did);
    println!("  To:   {} (Farming Collective)", coop1_did);
    println!("  Amount: 20 units");
    println!("  Memo: Weekly food box delivery");
    
    // Submit and process the second transaction
    processor.submit_transaction(tx2).await?;
    let results2 = processor.process_pending_transactions().await;
    
    // Check and display results
    if let Some(Ok(result)) = results2.first() {
        println!("\nSecond transaction completed successfully!");
        println!("  Transaction ID: {}", result.transaction.id);
        println!("  Status: Completed");
        println!("  Timestamp: {}", result.timestamp);
        
        // Display updated balances
        println!("\nUpdated balances:");
        for (did, balance) in &result.updated_balances {
            println!("  {}: {}", did, balance);
        }
    } else if let Some(Err(error)) = results2.first() {
        println!("\nSecond transaction failed: {}", error);
    }
    
    // Display final state
    let graph_lock = graph.lock().await;
    
    let coop1_account = graph_lock.get_account(&coop1_did).await?.unwrap();
    let coop2_account = graph_lock.get_account(&coop2_did).await?.unwrap();
    
    println!("\nFinal account balances:");
    println!("  {} (Farming Collective): {}", coop1_did, coop1_account.balance);
    println!("  {} (Tech Support): {}", coop2_did, coop2_account.balance);
    
    // Display credit line state
    let cl_id1 = CreditLineId::new(&coop1_did, &coop2_did);
    let cl_id2 = CreditLineId::new(&coop2_did, &coop1_did);
    
    let cl1 = graph_lock.get_credit_line(&cl_id1).await?.unwrap();
    let cl2 = graph_lock.get_credit_line(&cl_id2).await?.unwrap();
    
    println!("\nCredit line states:");
    println!("  {} → {}: Limit: {}, Balance: {}, Available: {}", 
        coop1_did, coop2_did, cl1.limit, cl1.balance, cl1.available_credit());
    println!("  {} → {}: Limit: {}, Balance: {}, Available: {}", 
        coop2_did, coop1_did, cl2.limit, cl2.balance, cl2.available_credit());
    
    println!("\n=== Example completed successfully ===");
    
    Ok(())
} 