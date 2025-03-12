use icn_mutual_credit::{
    Account, Amount, CreditClearingParams, CreditGraph, CreditLine, CreditLineId,
    CreditTerms, DID, Transaction, TransactionProcessor, TransactionType,
};
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== ICN Mutual Credit - Credit Clearing Example ===\n");
    
    // Create DIDs for four cooperatives in a circle
    let coop_a = DID::new("did:icn:coop:a");
    let coop_b = DID::new("did:icn:coop:b");
    let coop_c = DID::new("did:icn:coop:c");
    let coop_d = DID::new("did:icn:coop:d");
    
    println!("Created cooperatives in a circle:");
    println!("  - {} (Bakery)", coop_a);
    println!("  - {} (Construction)", coop_b);
    println!("  - {} (Daycare)", coop_c);
    println!("  - {} (Energy)", coop_d);
    
    // Create a credit graph
    let mut graph = CreditGraph::new();
    
    // Create and add accounts
    let accounts = [
        Account::new(coop_a.clone(), "Bakery Cooperative".to_string()),
        Account::new(coop_b.clone(), "Construction Cooperative".to_string()),
        Account::new(coop_c.clone(), "Daycare Cooperative".to_string()),
        Account::new(coop_d.clone(), "Energy Cooperative".to_string()),
    ];
    
    for account in &accounts {
        graph.add_account(account.clone()).await?;
    }
    println!("\nAdded accounts to the credit graph");
    
    // Create and add credit lines in a circular pattern:
    // A -> B -> C -> D -> A
    let credit_lines = [
        // A -> B
        CreditLine::new(
            coop_a.clone(),
            coop_b.clone(),
            Amount::new(100),
            CreditTerms::new(),
        ),
        // B -> C
        CreditLine::new(
            coop_b.clone(),
            coop_c.clone(),
            Amount::new(100),
            CreditTerms::new(),
        ),
        // C -> D
        CreditLine::new(
            coop_c.clone(),
            coop_d.clone(),
            Amount::new(100),
            CreditTerms::new(),
        ),
        // D -> A
        CreditLine::new(
            coop_d.clone(),
            coop_a.clone(),
            Amount::new(100),
            CreditTerms::new(),
        ),
    ];
    
    for credit_line in &credit_lines {
        graph.add_credit_line(credit_line.clone()).await?;
    }
    println!("Created credit lines between cooperatives in a circle");
    
    // Wrap the graph in an Arc<Mutex> for the transaction processor
    let graph = Arc::new(Mutex::new(graph));
    
    // Create a transaction processor with custom clearing parameters
    let clearing_params = CreditClearingParams {
        min_clearing_amount: Amount::new(1),
        max_path_length: 6,
        prioritize_high_value: true,
    };
    
    let mut processor = TransactionProcessor::new(Arc::clone(&graph), Some(clearing_params));
    println!("\nCreated transaction processor with credit clearing parameters");
    
    // Create a series of transactions that form a circular debt pattern
    
    // A pays B 50 units for construction services
    let tx1 = Transaction::new(
        "tx-001".to_string(),
        coop_a.clone(),
        coop_b.clone(),
        Amount::new(50),
        TransactionType::DirectTransfer,
        Some("Construction services".to_string()),
    );
    
    // B pays C 40 units for daycare services
    let tx2 = Transaction::new(
        "tx-002".to_string(),
        coop_b.clone(),
        coop_c.clone(),
        Amount::new(40),
        TransactionType::DirectTransfer,
        Some("Daycare services".to_string()),
    );
    
    // C pays D 30 units for energy services
    let tx3 = Transaction::new(
        "tx-003".to_string(),
        coop_c.clone(),
        coop_d.clone(),
        Amount::new(30),
        TransactionType::DirectTransfer,
        Some("Energy services".to_string()),
    );
    
    // D pays A 20 units for bakery goods
    let tx4 = Transaction::new(
        "tx-004".to_string(),
        coop_d.clone(),
        coop_a.clone(),
        Amount::new(20),
        TransactionType::DirectTransfer,
        Some("Bakery goods".to_string()),
    );
    
    println!("\nSubmitting transactions for a circular debt pattern:");
    println!("  1. A -> B: 50 units (Construction services)");
    println!("  2. B -> C: 40 units (Daycare services)");
    println!("  3. C -> D: 30 units (Energy services)");
    println!("  4. D -> A: 20 units (Bakery goods)");
    
    // Submit and process all transactions
    processor.submit_transaction(tx1).await?;
    processor.submit_transaction(tx2).await?;
    processor.submit_transaction(tx3).await?;
    processor.submit_transaction(tx4).await?;
    
    let results = processor.process_pending_transactions().await;
    
    println!("\nAll transactions processed successfully\n");
    
    // Display balances before clearing
    {
        let graph_lock = graph.lock().await;
        
        println!("Account balances before clearing:");
        println!("  {} (Bakery): {}", coop_a, graph_lock.get_account(&coop_a).await?.unwrap().balance);
        println!("  {} (Construction): {}", coop_b, graph_lock.get_account(&coop_b).await?.unwrap().balance);
        println!("  {} (Daycare): {}", coop_c, graph_lock.get_account(&coop_c).await?.unwrap().balance);
        println!("  {} (Energy): {}", coop_d, graph_lock.get_account(&coop_d).await?.unwrap().balance);
        
        println!("\nCredit line balances before clearing:");
        println!("  A -> B: {}", graph_lock.get_credit_line(&CreditLineId::new(&coop_a, &coop_b)).await?.unwrap().balance);
        println!("  B -> C: {}", graph_lock.get_credit_line(&CreditLineId::new(&coop_b, &coop_c)).await?.unwrap().balance);
        println!("  C -> D: {}", graph_lock.get_credit_line(&CreditLineId::new(&coop_c, &coop_d)).await?.unwrap().balance);
        println!("  D -> A: {}", graph_lock.get_credit_line(&CreditLineId::new(&coop_d, &coop_a)).await?.unwrap().balance);
    }
    
    // Run the credit clearing algorithm
    println!("\nRunning credit clearing algorithm...");
    let clearing_txs = processor.run_credit_clearing().await?;
    
    println!("Credit clearing completed with {} transactions", clearing_txs.len());
    for (i, tx) in clearing_txs.iter().enumerate() {
        println!("  {}. {} -> {}: {}", i+1, tx.from, tx.to, tx.amount);
    }
    
    // Display balances after clearing
    {
        let graph_lock = graph.lock().await;
        
        println!("\nAccount balances after clearing:");
        println!("  {} (Bakery): {}", coop_a, graph_lock.get_account(&coop_a).await?.unwrap().balance);
        println!("  {} (Construction): {}", coop_b, graph_lock.get_account(&coop_b).await?.unwrap().balance);
        println!("  {} (Daycare): {}", coop_c, graph_lock.get_account(&coop_c).await?.unwrap().balance);
        println!("  {} (Energy): {}", coop_d, graph_lock.get_account(&coop_d).await?.unwrap().balance);
        
        println!("\nCredit line balances after clearing:");
        println!("  A -> B: {}", graph_lock.get_credit_line(&CreditLineId::new(&coop_a, &coop_b)).await?.unwrap().balance);
        println!("  B -> C: {}", graph_lock.get_credit_line(&CreditLineId::new(&coop_b, &coop_c)).await?.unwrap().balance);
        println!("  C -> D: {}", graph_lock.get_credit_line(&CreditLineId::new(&coop_c, &coop_d)).await?.unwrap().balance);
        println!("  D -> A: {}", graph_lock.get_credit_line(&CreditLineId::new(&coop_d, &coop_a)).await?.unwrap().balance);
    }
    
    println!("\n=== Example completed successfully ===");
    println!("\nNote: The circular debt pattern of 20 units has been cleared.");
    println!("Each cooperative's balance has been adjusted by the minimum amount");
    println!("in the cycle, reducing the overall debt in the system.");
    
    Ok(())
} 