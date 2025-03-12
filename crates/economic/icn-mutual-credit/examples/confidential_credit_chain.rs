//! Example demonstrating confidential transactions in a multi-party credit chain scenario
//! 
//! This example shows how confidential transactions can be used in a more complex
//! scenario involving multiple participants in a supply chain or service network.

use icn_mutual_credit::{
    Account, Amount, CreditGraph, CreditLine, CreditTerms,
    TransactionProcessor, DID, Transaction, TransactionStatus, TransactionType,
};

use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔒 Confidential Credit Chain Example 🔒");
    println!("====================================\n");
    
    // Create a sample credit graph for a multi-party supply chain
    let mut graph = CreditGraph::new();
    
    // Create participants in the supply chain
    let producer_did = DID::new("did:icn:producer");
    let manufacturer_did = DID::new("did:icn:manufacturer");
    let distributor_did = DID::new("did:icn:distributor");
    let retailer_did = DID::new("did:icn:retailer");
    
    println!("🏭 Creating accounts for supply chain participants:");
    println!("  - Producer (raw materials)");
    println!("  - Manufacturer (processes raw materials)");
    println!("  - Distributor (handles logistics)");
    println!("  - Retailer (sells to consumers)\n");
    
    // Create accounts for each participant
    let producer = Account::new(producer_did.clone(), "Raw Materials Producer".to_string());
    let manufacturer = Account::new(manufacturer_did.clone(), "Product Manufacturer".to_string());
    let distributor = Account::new(distributor_did.clone(), "Product Distributor".to_string());
    let retailer = Account::new(retailer_did.clone(), "Retail Store".to_string());
    
    // Add accounts to the graph
    graph.add_account(producer).await?;
    graph.add_account(manufacturer).await?;
    graph.add_account(distributor).await?;
    graph.add_account(retailer).await?;
    
    println!("✅ All accounts created successfully\n");
    
    // Establish credit lines between participants in the supply chain
    println!("🔄 Establishing credit lines in the supply chain");
    
    // Credit line from Manufacturer to Producer (manufacturer pays for raw materials)
    let manufacturer_to_producer = CreditLine::new(
        manufacturer_did.clone(),
        producer_did.clone(),
        Amount::new(5000),  // Higher limit for raw materials
        CreditTerms::new(),
    );
    
    // Credit line from Distributor to Manufacturer (distributor pays for products)
    let distributor_to_manufacturer = CreditLine::new(
        distributor_did.clone(),
        manufacturer_did.clone(),
        Amount::new(8000),  // Higher limit for finished products
        CreditTerms::new(),
    );
    
    // Credit line from Retailer to Distributor (retailer pays for distribution)
    let retailer_to_distributor = CreditLine::new(
        retailer_did.clone(),
        distributor_did.clone(),
        Amount::new(6000),  // Credit limit for distribution services
        CreditTerms::new(),
    );
    
    // Add credit lines to the graph
    graph.add_credit_line(manufacturer_to_producer).await?;
    graph.add_credit_line(distributor_to_manufacturer).await?;
    graph.add_credit_line(retailer_to_distributor).await?;
    
    println!("✅ Credit lines established\n");
    
    // Create a transaction processor with the graph
    let graph_arc = Arc::new(Mutex::new(graph));
    let mut processor = TransactionProcessor::new(graph_arc.clone(), None);
    
    // Start a confidential transaction chain
    println!("🔒 Initiating confidential transaction chain:");
    println!("  1. Retailer → Distributor (payment for logistics)");
    println!("  2. Distributor → Manufacturer (payment for products)");
    println!("  3. Manufacturer → Producer (payment for raw materials)\n");
    
    // Step 1: Retailer pays Distributor for logistics services (confidential)
    println!("📝 Step 1: Retailer pays Distributor");
    let retailer_payment = Amount::new(2000);
    let tx_id1 = processor.create_confidential_transaction(
        &retailer_did,
        &distributor_did,
        retailer_payment.clone(),
        Some("Payment for distribution services".to_string()),
    ).await?;
    
    // Process the transaction
    processor.process_pending_transactions().await;
    
    println!("  - Transaction ID: {}", tx_id1);
    println!("  - Amount: {} (confidential in production)\n", retailer_payment);
    
    // Step 2: Distributor pays Manufacturer for products (confidential)
    println!("📝 Step 2: Distributor pays Manufacturer");
    let distributor_payment = Amount::new(3500);
    let tx_id2 = processor.create_confidential_transaction(
        &distributor_did,
        &manufacturer_did,
        distributor_payment.clone(),
        Some("Payment for manufactured products".to_string()),
    ).await?;
    
    // Process the transaction
    processor.process_pending_transactions().await;
    
    println!("  - Transaction ID: {}", tx_id2);
    println!("  - Amount: {} (confidential in production)\n", distributor_payment);
    
    // Step 3: Manufacturer pays Producer for raw materials (confidential)
    println!("📝 Step 3: Manufacturer pays Producer");
    let manufacturer_payment = Amount::new(1800);
    let tx_id3 = processor.create_confidential_transaction(
        &manufacturer_did,
        &producer_did,
        manufacturer_payment.clone(),
        Some("Payment for raw materials".to_string()),
    ).await?;
    
    // Process the transaction
    processor.process_pending_transactions().await;
    
    println!("  - Transaction ID: {}", tx_id3);
    println!("  - Amount: {} (confidential in production)\n", manufacturer_payment);
    
    // Display the final balances
    let graph_lock = graph_arc.lock().await;
    
    println!("📊 Final Account Balances:");
    
    let producer_balance = graph_lock.get_account_balance(&producer_did).await?;
    let manufacturer_balance = graph_lock.get_account_balance(&manufacturer_did).await?;
    let distributor_balance = graph_lock.get_account_balance(&distributor_did).await?;
    let retailer_balance = graph_lock.get_account_balance(&retailer_did).await?;
    
    println!("  Producer: {} (received payment for materials)", producer_balance);
    println!("  Manufacturer: {} (paid for materials, received payment for products)", manufacturer_balance);
    println!("  Distributor: {} (paid for products, received payment for services)", distributor_balance);
    println!("  Retailer: {} (paid for distribution services)", retailer_balance);
    
    drop(graph_lock);
    
    // In a real system, balances and transaction amounts would be confidential
    // and would only be visible to authorized parties with the proper blinding factors
    
    println!("\n✅ Example completed successfully");
    
    // In this example, we've demonstrated how confidential transactions can be used
    // in a multi-party supply chain, where each participant can transact without
    // revealing sensitive pricing information to the broader network.
    
    Ok(())
} 