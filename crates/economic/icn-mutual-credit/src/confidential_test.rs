//! Tests for confidential transactions

use std::sync::Arc;
use tokio::sync::Mutex;
use rust_decimal::Decimal;

use crate::{
    Account, AccountStatus, Amount, CreditGraph, CreditLine, CreditTerms,
    DID, Transaction, TransactionProcessor, TransactionStatus, TransactionType,
    confidential::{
        ConfidentialTransaction, ConfidentialTransactionProcessor,
        PedersenCommitment, RangeProof, BlindingFactor, ConfidentialError
    }
};

#[tokio::test]
async fn test_confidential_transaction_end_to_end() {
    // Create a credit graph with two accounts
    let mut graph = CreditGraph::new();
    
    // Create test accounts
    let alice_did = DID::new("did:icn:alice");
    let bob_did = DID::new("did:icn:bob");
    
    let alice = Account::new(
        alice_did.clone(),
        "Alice".to_string(),
    );
    
    let bob = Account::new(
        bob_did.clone(),
        "Bob".to_string(),
    );
    
    // Add accounts to the graph
    graph.add_account(alice).await.unwrap();
    graph.add_account(bob).await.unwrap();
    
    // Create a credit line between Alice and Bob
    let credit_line = CreditLine::new(
        alice_did.clone(),
        bob_did.clone(),
        Amount::new(1000),
        CreditTerms::new(),
    );
    
    graph.add_credit_line(credit_line).await.unwrap();
    
    // Create a transaction processor
    let graph_arc = Arc::new(Mutex::new(graph));
    let mut processor = TransactionProcessor::new(graph_arc.clone(), None);
    
    // Create a confidential transaction from Alice to Bob
    let amount = Amount::new(500);
    let tx_id = processor.create_confidential_transaction(
        &alice_did,
        &bob_did,
        amount.clone(),
        Some("Confidential payment".to_string()),
    ).await.unwrap();
    
    // Process all pending transactions
    let results = processor.process_pending_transactions().await;
    // All transactions should be successful
    for result in &results {
        assert!(result.is_ok());
    }
    
    // Check transaction history
    let history = processor.get_transaction_history();
    assert_eq!(history.len(), 1);
    
    let tx_result = &history[0];
    assert_eq!(tx_result.transaction.id, tx_id);
    assert_eq!(tx_result.transaction.status, TransactionStatus::Completed);
    
    // Verify account balances
    let graph_lock = graph_arc.lock().await;
    
    // Check Alice's balance (should be negative since she sent money)
    let alice_balance = graph_lock.get_account_balance(&alice_did).await.unwrap();
    assert_eq!(alice_balance, Amount::new(-500));
    
    // Check Bob's balance (should be positive since he received money)
    let bob_balance = graph_lock.get_account_balance(&bob_did).await.unwrap();
    assert_eq!(bob_balance, Amount::new(500));
    
    drop(graph_lock);
    
    // Reveal the amount of the confidential transaction (only in test mode)
    #[cfg(test)]
    {
        let revealed_amount = processor.reveal_confidential_amount(&tx_id).unwrap();
        assert_eq!(revealed_amount, amount);
    }
}

#[tokio::test]
async fn test_confidential_transaction_with_multiple_transfers() {
    // Create a credit graph with three accounts
    let mut graph = CreditGraph::new();
    
    // Create test accounts
    let alice_did = DID::new("did:icn:alice");
    let bob_did = DID::new("did:icn:bob");
    let charlie_did = DID::new("did:icn:charlie");
    
    let alice = Account::new(alice_did.clone(), "Alice".to_string());
    let bob = Account::new(bob_did.clone(), "Bob".to_string());
    let charlie = Account::new(charlie_did.clone(), "Charlie".to_string());
    
    // Add accounts to the graph
    graph.add_account(alice).await.unwrap();
    graph.add_account(bob).await.unwrap();
    graph.add_account(charlie).await.unwrap();
    
    // Create credit lines between accounts
    let alice_to_bob = CreditLine::new(
        alice_did.clone(),
        bob_did.clone(),
        Amount::new(1000),
        CreditTerms::new(),
    );
    
    let bob_to_charlie = CreditLine::new(
        bob_did.clone(),
        charlie_did.clone(),
        Amount::new(1000),
        CreditTerms::new(),
    );
    
    graph.add_credit_line(alice_to_bob).await.unwrap();
    graph.add_credit_line(bob_to_charlie).await.unwrap();
    
    // Create a transaction processor
    let graph_arc = Arc::new(Mutex::new(graph));
    let mut processor = TransactionProcessor::new(graph_arc.clone(), None);
    
    // First - create a confidential transaction from Alice to Bob
    let amount1 = Amount::new(300);
    let tx_id1 = processor.create_confidential_transaction(
        &alice_did,
        &bob_did,
        amount1.clone(),
        Some("Confidential payment 1".to_string()),
    ).await.unwrap();
    
    // Process pending transactions
    let results = processor.process_pending_transactions().await;
    for result in &results {
        assert!(result.is_ok());
    }
    
    // Second - create a confidential transaction from Bob to Charlie
    let amount2 = Amount::new(200);
    let tx_id2 = processor.create_confidential_transaction(
        &bob_did,
        &charlie_did,
        amount2.clone(),
        Some("Confidential payment 2".to_string()),
    ).await.unwrap();
    
    // Process pending transactions
    let results = processor.process_pending_transactions().await;
    for result in &results {
        assert!(result.is_ok());
    }
    
    // Check transaction history
    let history = processor.get_transaction_history();
    assert_eq!(history.len(), 2);
    
    // Verify account balances
    let graph_lock = graph_arc.lock().await;
    
    // Alice sent 300 to Bob
    let alice_balance = graph_lock.get_account_balance(&alice_did).await.unwrap();
    assert_eq!(alice_balance, Amount::new(-300));
    
    // Bob received 300 from Alice and sent 200 to Charlie
    let bob_balance = graph_lock.get_account_balance(&bob_did).await.unwrap();
    assert_eq!(bob_balance, Amount::new(100)); // 300 - 200 = 100
    
    // Charlie received 200 from Bob
    let charlie_balance = graph_lock.get_account_balance(&charlie_did).await.unwrap();
    assert_eq!(charlie_balance, Amount::new(200));
    
    drop(graph_lock);
    
    // Reveal the amount of the confidential transactions (only in test mode)
    #[cfg(test)]
    {
        let revealed_amount1 = processor.reveal_confidential_amount(&tx_id1).unwrap();
        assert_eq!(revealed_amount1, amount1);
        
        let revealed_amount2 = processor.reveal_confidential_amount(&tx_id2).unwrap();
        assert_eq!(revealed_amount2, amount2);
    }
}

#[test]
fn test_pedersen_commitment() {
    let generator = crate::confidential::PedersenCommitmentGenerator::new();
    
    // Create a blinding factor
    let blinding_factor = generator.generate_blinding_factor();
    
    // Create a commitment to a value
    let amount = 500;
    let commitment = generator.create_commitment(amount, &blinding_factor).unwrap();
    
    // Verify the commitment
    let is_valid = generator.verify_commitment(&commitment, amount, &blinding_factor).unwrap();
    assert!(is_valid);
    
    // Verify that an incorrect amount produces an invalid result
    let is_invalid = generator.verify_commitment(&commitment, amount + 1, &blinding_factor).unwrap();
    assert!(!is_invalid);
}

#[test]
fn test_range_proof() {
    let range_proof_system = crate::confidential::RangeProofSystem::new();
    
    // Create a blinding factor
    let blinding_factor = crate::confidential::BlindingFactor::new().unwrap();
    
    // Create a range proof for a value in the range [0, 1000]
    let amount = 500;
    let range_proof = range_proof_system.create_range_proof(
        amount,
        0,
        1000,
        &blinding_factor,
    ).unwrap();
    
    // Create a commitment to verify against
    let generator = crate::confidential::PedersenCommitmentGenerator::new();
    let commitment = generator.create_commitment(amount, &blinding_factor).unwrap();
    
    // Verify the range proof
    let is_valid = range_proof_system.verify_range_proof(&range_proof, &commitment).unwrap();
    assert!(is_valid);
    
    // Test that a value outside the range will fail to create a range proof
    let out_of_range = range_proof_system.create_range_proof(
        1500, // Outside the range
        0,
        1000,
        &blinding_factor,
    );
    assert!(out_of_range.is_err());
    
    if let Err(ConfidentialError::AmountRangeError(_)) = out_of_range {
        // This is the expected error type
    } else {
        panic!("Expected AmountRangeError");
    }
} 