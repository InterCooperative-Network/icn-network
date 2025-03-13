use std::sync::Arc;
use tempfile::tempdir;
use crate::federation::*;
use crate::identity::Identity;
use crate::storage::Storage;
use crate::crypto::CryptoUtils;

fn setup_test() -> (FederationSystem, tempfile::TempDir) {
    let temp_dir = tempdir().unwrap();
    let storage = Storage::new(temp_dir.path().to_path_buf());
    let identity = Identity::new(
        "test-coop".to_string(),
        "test-node".to_string(),
        "test-did:test:test-coop:test-node".to_string(),
        storage.clone(),
    ).unwrap();
    let federation_system = FederationSystem::new(identity, storage);
    (federation_system, temp_dir)
}

#[test]
fn test_create_federation() {
    let (federation_system, _temp_dir) = setup_test();

    let policies = FederationPolicies {
        max_transaction_amount: 1000,
        min_transaction_amount: 1,
        max_credit_limit: 5000,
        min_credit_limit: 100,
        transaction_fee: 1,
        settlement_period: 3600,
    };

    let federation = federation_system
        .create_federation("Test Federation", Some("Test Description".to_string()), policies)
        .unwrap();

    assert_eq!(federation.name, "Test Federation");
    assert_eq!(federation.description.unwrap(), "Test Description");
    assert_eq!(federation.members.len(), 1);
    assert_eq!(federation.members[0].cooperative_id, "test-coop");
    assert_eq!(federation.members[0].credit_limit, 5000);
}

#[test]
fn test_join_federation() {
    let (federation_system, _temp_dir) = setup_test();

    // Create a federation first
    let policies = FederationPolicies {
        max_transaction_amount: 1000,
        min_transaction_amount: 1,
        max_credit_limit: 5000,
        min_credit_limit: 100,
        transaction_fee: 1,
        settlement_period: 3600,
    };

    let federation = federation_system
        .create_federation("Test Federation", None, policies)
        .unwrap();

    // Create another identity to join the federation
    let temp_dir = tempdir().unwrap();
    let storage = Storage::new(temp_dir.path().to_path_buf());
    let identity = Identity::new(
        "other-coop".to_string(),
        "other-node".to_string(),
        "test-did:test:other-coop:other-node".to_string(),
        storage.clone(),
    ).unwrap();
    let other_federation_system = FederationSystem::new(identity, storage);

    // Join the federation
    other_federation_system.join_federation(&federation.id, 2000).unwrap();

    // Verify the new member was added
    let updated_federation: Federation = federation_system.storage
        .get_json(&format!("federations/{}", federation.id))
        .unwrap();
    assert_eq!(updated_federation.members.len(), 2);
    assert!(updated_federation.members.iter().any(|m| m.cooperative_id == "other-coop"));
}

#[test]
fn test_create_federation_transaction() {
    let (federation_system, _temp_dir) = setup_test();

    // Create a federation
    let policies = FederationPolicies {
        max_transaction_amount: 1000,
        min_transaction_amount: 1,
        max_credit_limit: 5000,
        min_credit_limit: 100,
        transaction_fee: 1,
        settlement_period: 3600,
    };

    let federation = federation_system
        .create_federation("Test Federation", None, policies)
        .unwrap();

    // Create test accounts
    let from_did = "test-did:test:test-coop:member1";
    let to_did = "test-did:test:test-coop:member2";
    
    let mut from_account = MemberAccount {
        did: from_did.to_string(),
        cooperative: "test-coop".to_string(),
        balance: 1000,
        credit_limit: 5000,
        last_updated: 0,
        transactions: vec![],
    };
    
    let mut to_account = MemberAccount {
        did: to_did.to_string(),
        cooperative: "test-coop".to_string(),
        balance: 0,
        credit_limit: 5000,
        last_updated: 0,
        transactions: vec![],
    };

    federation_system.storage.put_json(
        &format!("members/{}", from_did),
        &from_account,
    ).unwrap();
    federation_system.storage.put_json(
        &format!("members/{}", to_did),
        &to_account,
    ).unwrap();

    // Create a transaction
    let transaction = federation_system
        .create_federation_transaction(
            &federation.id,
            from_did,
            to_did,
            100,
            Some("Test transaction".to_string()),
        )
        .unwrap();

    assert_eq!(transaction.federation_id, federation.id);
    assert_eq!(transaction.from_did, from_did);
    assert_eq!(transaction.to_did, to_did);
    assert_eq!(transaction.amount, 100);
    assert_eq!(transaction.description.unwrap(), "Test transaction");
    assert_eq!(transaction.status, FederationTransactionStatus::Pending);
}

#[test]
fn test_process_federation_transaction() {
    let (federation_system, _temp_dir) = setup_test();

    // Create a federation
    let policies = FederationPolicies {
        max_transaction_amount: 1000,
        min_transaction_amount: 1,
        max_credit_limit: 5000,
        min_credit_limit: 100,
        transaction_fee: 1,
        settlement_period: 3600,
    };

    let federation = federation_system
        .create_federation("Test Federation", None, policies)
        .unwrap();

    // Create test accounts
    let from_did = "test-did:test:test-coop:member1";
    let to_did = "test-did:test:test-coop:member2";
    
    let mut from_account = MemberAccount {
        did: from_did.to_string(),
        cooperative: "test-coop".to_string(),
        balance: 1000,
        credit_limit: 5000,
        last_updated: 0,
        transactions: vec![],
    };
    
    let mut to_account = MemberAccount {
        did: to_did.to_string(),
        cooperative: "test-coop".to_string(),
        balance: 0,
        credit_limit: 5000,
        last_updated: 0,
        transactions: vec![],
    };

    federation_system.storage.put_json(
        &format!("members/{}", from_did),
        &from_account,
    ).unwrap();
    federation_system.storage.put_json(
        &format!("members/{}", to_did),
        &to_account,
    ).unwrap();

    // Create and process a transaction
    let transaction = federation_system
        .create_federation_transaction(
            &federation.id,
            from_did,
            to_did,
            100,
            Some("Test transaction".to_string()),
        )
        .unwrap();

    federation_system.process_federation_transaction(&transaction).unwrap();

    // Verify account balances were updated
    let updated_from_account: MemberAccount = federation_system.storage
        .get_json(&format!("members/{}", from_did))
        .unwrap();
    let updated_to_account: MemberAccount = federation_system.storage
        .get_json(&format!("members/{}", to_did))
        .unwrap();

    assert_eq!(updated_from_account.balance, 899); // 1000 - 100 - 1 (fee)
    assert_eq!(updated_to_account.balance, 100);
    assert_eq!(updated_from_account.transactions.len(), 1);
    assert_eq!(updated_to_account.transactions.len(), 1);

    // Verify transaction status was updated
    let updated_transaction: FederationTransaction = federation_system.storage
        .get_json(&format!("federation_transactions/{}", transaction.id))
        .unwrap();
    assert_eq!(updated_transaction.status, FederationTransactionStatus::Completed);
}

#[test]
fn test_get_federation_transactions() {
    let (federation_system, _temp_dir) = setup_test();

    // Create a federation
    let policies = FederationPolicies {
        max_transaction_amount: 1000,
        min_transaction_amount: 1,
        max_credit_limit: 5000,
        min_credit_limit: 100,
        transaction_fee: 1,
        settlement_period: 3600,
    };

    let federation = federation_system
        .create_federation("Test Federation", None, policies)
        .unwrap();

    // Create test accounts
    let from_did = "test-did:test:test-coop:member1";
    let to_did = "test-did:test:test-coop:member2";
    
    let mut from_account = MemberAccount {
        did: from_did.to_string(),
        cooperative: "test-coop".to_string(),
        balance: 1000,
        credit_limit: 5000,
        last_updated: 0,
        transactions: vec![],
    };
    
    let mut to_account = MemberAccount {
        did: to_did.to_string(),
        cooperative: "test-coop".to_string(),
        balance: 0,
        credit_limit: 5000,
        last_updated: 0,
        transactions: vec![],
    };

    federation_system.storage.put_json(
        &format!("members/{}", from_did),
        &from_account,
    ).unwrap();
    federation_system.storage.put_json(
        &format!("members/{}", to_did),
        &to_account,
    ).unwrap();

    // Create multiple transactions
    let transaction1 = federation_system
        .create_federation_transaction(
            &federation.id,
            from_did,
            to_did,
            100,
            Some("Transaction 1".to_string()),
        )
        .unwrap();

    let transaction2 = federation_system
        .create_federation_transaction(
            &federation.id,
            from_did,
            to_did,
            200,
            Some("Transaction 2".to_string()),
        )
        .unwrap();

    // Get all transactions
    let transactions = federation_system
        .get_federation_transactions(&federation.id)
        .unwrap();

    assert_eq!(transactions.len(), 2);
    assert!(transactions.iter().any(|t| t.id == transaction1.id));
    assert!(transactions.iter().any(|t| t.id == transaction2.id));
}

#[test]
fn test_get_federation_members() {
    let (federation_system, _temp_dir) = setup_test();

    // Create a federation
    let policies = FederationPolicies {
        max_transaction_amount: 1000,
        min_transaction_amount: 1,
        max_credit_limit: 5000,
        min_credit_limit: 100,
        transaction_fee: 1,
        settlement_period: 3600,
    };

    let federation = federation_system
        .create_federation("Test Federation", None, policies)
        .unwrap();

    // Create another identity to join the federation
    let temp_dir = tempdir().unwrap();
    let storage = Storage::new(temp_dir.path().to_path_buf());
    let identity = Identity::new(
        "other-coop".to_string(),
        "other-node".to_string(),
        "test-did:test:other-coop:other-node".to_string(),
        storage.clone(),
    ).unwrap();
    let other_federation_system = FederationSystem::new(identity, storage);

    // Join the federation
    other_federation_system.join_federation(&federation.id, 2000).unwrap();

    // Get federation members
    let members = federation_system
        .get_federation_members(&federation.id)
        .unwrap();

    assert_eq!(members.len(), 2);
    assert!(members.iter().any(|m| m.cooperative_id == "test-coop"));
    assert!(members.iter().any(|m| m.cooperative_id == "other-coop"));
} 