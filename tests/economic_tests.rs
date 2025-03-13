use std::sync::Arc;
use tempfile::tempdir;
use crate::config::NodeConfig;
use crate::identity::Identity;
use crate::storage::Storage;
use crate::economic::{MutualCreditSystem, Transaction};

#[test]
fn test_register_member() {
    let dir = tempdir().unwrap();
    let storage = Storage::new(dir.path().to_str().unwrap()).unwrap();
    let identity = Identity::new("test-node", "test-coop").unwrap();
    let economic = MutualCreditSystem::new(identity, storage);

    let member_did = "did:icn:test-coop:member-1";
    let account = economic.register_member(member_did, 1000).unwrap();
    assert_eq!(account.balance, 0);
    assert_eq!(account.credit_limit, 1000);
    assert_eq!(account.did, member_did);
    assert_eq!(account.cooperative, "test-coop");
}

#[test]
fn test_create_transaction() {
    let dir = tempdir().unwrap();
    let storage = Storage::new(dir.path().to_str().unwrap()).unwrap();
    let identity = Identity::new("test-node", "test-coop").unwrap();
    let economic = MutualCreditSystem::new(identity, storage);

    // Register members
    let member1_did = "did:icn:test-coop:member-1";
    let member2_did = "did:icn:test-coop:member-2";
    economic.register_member(member1_did, 1000).unwrap();
    economic.register_member(member2_did, 1000).unwrap();

    // Create transaction
    let transaction = economic.create_transaction(
        member1_did,
        member2_did,
        100,
        Some("Test transaction".to_string()),
    ).unwrap();

    assert_eq!(transaction.amount, 100);
    assert_eq!(transaction.from_did, member1_did);
    assert_eq!(transaction.to_did, member2_did);
    assert!(transaction.description.is_some());
    assert!(!transaction.signature.is_empty());
    assert_eq!(transaction.cooperative, "test-coop");

    // Check balance
    let balance = economic.get_member_balance(member1_did).unwrap();
    assert_eq!(balance, -100);
}

#[test]
fn test_insufficient_credit() {
    let dir = tempdir().unwrap();
    let storage = Storage::new(dir.path().to_str().unwrap()).unwrap();
    let identity = Identity::new("test-node", "test-coop").unwrap();
    let economic = MutualCreditSystem::new(identity, storage);

    // Register member with low limit
    let member1_did = "did:icn:test-coop:member-1";
    let member2_did = "did:icn:test-coop:member-2";
    economic.register_member(member1_did, 100).unwrap();
    economic.register_member(member2_did, 1000).unwrap();

    // Try to create transaction exceeding limit
    let result = economic.create_transaction(
        member1_did,
        member2_did,
        200,
        None,
    );

    assert!(result.is_err());
}

#[test]
fn test_process_transaction() {
    let dir = tempdir().unwrap();
    let storage = Storage::new(dir.path().to_str().unwrap()).unwrap();
    let identity = Identity::new("test-node", "test-coop").unwrap();
    let economic = MutualCreditSystem::new(identity, storage);

    // Register members
    let member1_did = "did:icn:test-coop:member-1";
    let member2_did = "did:icn:test-coop:member-2";
    economic.register_member(member1_did, 1000).unwrap();
    economic.register_member(member2_did, 1000).unwrap();

    // Create and process transaction
    let transaction = economic.create_transaction(
        member1_did,
        member2_did,
        100,
        None,
    ).unwrap();

    economic.process_transaction(&transaction).unwrap();

    // Check balances
    let balance1 = economic.get_member_balance(member1_did).unwrap();
    let balance2 = economic.get_member_balance(member2_did).unwrap();
    assert_eq!(balance1, -100);
    assert_eq!(balance2, 100);
}

#[test]
fn test_transaction_history() {
    let dir = tempdir().unwrap();
    let storage = Storage::new(dir.path().to_str().unwrap()).unwrap();
    let identity = Identity::new("test-node", "test-coop").unwrap();
    let economic = MutualCreditSystem::new(identity, storage);

    // Register members
    let member1_did = "did:icn:test-coop:member-1";
    let member2_did = "did:icn:test-coop:member-2";
    economic.register_member(member1_did, 1000).unwrap();
    economic.register_member(member2_did, 1000).unwrap();

    // Create multiple transactions
    economic.create_transaction(
        member1_did,
        member2_did,
        100,
        Some("First transaction".to_string()),
    ).unwrap();

    economic.create_transaction(
        member1_did,
        member2_did,
        200,
        Some("Second transaction".to_string()),
    ).unwrap();

    // Get transaction history
    let history = economic.get_member_transaction_history(member1_did).unwrap();
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].amount, 100);
    assert_eq!(history[1].amount, 200);
}

#[test]
fn test_invalid_member() {
    let dir = tempdir().unwrap();
    let storage = Storage::new(dir.path().to_str().unwrap()).unwrap();
    let identity = Identity::new("test-node", "test-coop").unwrap();
    let economic = MutualCreditSystem::new(identity, storage);

    // Try to register member from different cooperative
    let result = economic.register_member(
        "did:icn:other-coop:member-1",
        1000,
    );

    assert!(result.is_err());
}

#[test]
fn test_get_cooperative_members() {
    let dir = tempdir().unwrap();
    let storage = Storage::new(dir.path().to_str().unwrap()).unwrap();
    let identity = Identity::new("test-node", "test-coop").unwrap();
    let economic = MutualCreditSystem::new(identity, storage);

    // Register multiple members
    let member1_did = "did:icn:test-coop:member-1";
    let member2_did = "did:icn:test-coop:member-2";
    economic.register_member(member1_did, 1000).unwrap();
    economic.register_member(member2_did, 1000).unwrap();

    // Get all members
    let members = economic.get_cooperative_members().unwrap();
    assert_eq!(members.len(), 2);
    assert!(members.iter().any(|m| m.did == member1_did));
    assert!(members.iter().any(|m| m.did == member2_did));
} 