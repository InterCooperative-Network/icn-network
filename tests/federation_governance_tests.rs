use std::sync::Arc;
use tempfile::tempdir;
use crate::federation_governance::*;
use crate::identity::Identity;
use crate::storage::Storage;
use crate::crypto::CryptoUtils;

fn setup_test() -> (FederationGovernance, tempfile::TempDir) {
    let temp_dir = tempdir().unwrap();
    let storage = Storage::new(temp_dir.path().to_path_buf());
    let identity = Identity::new(
        "test-coop".to_string(),
        "test-node".to_string(),
        "test-did:test:test-coop:test-node".to_string(),
        storage.clone(),
    ).unwrap();
    let governance = FederationGovernance::new(identity, storage);
    (governance, temp_dir)
}

#[test]
fn test_create_proposal() {
    let (governance, _temp_dir) = setup_test();

    let changes = serde_json::json!({
        "max_transaction_amount": 2000,
        "transaction_fee": 2
    });

    let proposal = governance
        .create_proposal(
            "test-federation",
            ProposalType::PolicyChange,
            "Increase Transaction Limits",
            "Proposal to increase maximum transaction amount and fees",
            3600, // 1 hour voting period
            3,    // 3 votes required for quorum
            changes,
        )
        .unwrap();

    assert_eq!(proposal.federation_id, "test-federation");
    assert_eq!(proposal.proposal_type, ProposalType::PolicyChange);
    assert_eq!(proposal.title, "Increase Transaction Limits");
    assert_eq!(proposal.quorum, 3);
    assert_eq!(proposal.status, ProposalStatus::Active);
    assert!(proposal.votes.is_empty());
}

#[test]
fn test_vote_on_proposal() {
    let (governance, _temp_dir) = setup_test();

    // Create a proposal first
    let changes = serde_json::json!({
        "max_transaction_amount": 2000,
        "transaction_fee": 2
    });

    let proposal = governance
        .create_proposal(
            "test-federation",
            ProposalType::PolicyChange,
            "Test Proposal",
            "Test Description",
            3600,
            3,
            changes,
        )
        .unwrap();

    // Vote on the proposal
    governance.vote(&proposal.id, true).unwrap();

    // Verify the vote was recorded
    let updated_proposal: Proposal = governance.storage
        .get_json(&format!("proposals/{}", proposal.id))
        .unwrap();
    
    assert_eq!(updated_proposal.votes.len(), 1);
    assert_eq!(updated_proposal.votes[0].member_did, governance.identity.did);
    assert!(updated_proposal.votes[0].vote);
}

#[test]
fn test_process_proposal() {
    let (governance, _temp_dir) = setup_test();

    // Create a proposal first
    let changes = serde_json::json!({
        "max_transaction_amount": 2000,
        "transaction_fee": 2
    });

    let proposal = governance
        .create_proposal(
            "test-federation",
            ProposalType::PolicyChange,
            "Test Proposal",
            "Test Description",
            1, // 1 second voting period
            1, // 1 vote required for quorum
            changes,
        )
        .unwrap();

    // Vote on the proposal
    governance.vote(&proposal.id, true).unwrap();

    // Wait for voting period to end
    std::thread::sleep(std::time::Duration::from_secs(2));

    // Process the proposal
    governance.process_proposal(&proposal.id).unwrap();

    // Verify the proposal was processed
    let updated_proposal: Proposal = governance.storage
        .get_json(&format!("proposals/{}", proposal.id))
        .unwrap();
    
    assert_eq!(updated_proposal.status, ProposalStatus::Passed);
}

#[test]
fn test_create_dispute() {
    let (governance, _temp_dir) = setup_test();

    let evidence = vec![Evidence {
        id: "test-evidence".to_string(),
        submitted_by: governance.identity.did.clone(),
        timestamp: 0,
        description: "Test evidence".to_string(),
        data: vec![1, 2, 3],
        signature: vec![4, 5, 6],
    }];

    let dispute = governance
        .create_dispute(
            "test-federation",
            "test-transaction",
            "test-did:test:other-coop:other-node",
            "Test dispute description",
            evidence,
        )
        .unwrap();

    assert_eq!(dispute.federation_id, "test-federation");
    assert_eq!(dispute.transaction_id, "test-transaction");
    assert_eq!(dispute.complainant_did, governance.identity.did);
    assert_eq!(dispute.respondent_did, "test-did:test:other-coop:other-node");
    assert_eq!(dispute.status, DisputeStatus::Open);
    assert_eq!(dispute.evidence.len(), 1);
    assert!(dispute.resolution.is_none());
}

#[test]
fn test_add_evidence() {
    let (governance, _temp_dir) = setup_test();

    // Create a dispute first
    let dispute = governance
        .create_dispute(
            "test-federation",
            "test-transaction",
            "test-did:test:other-coop:other-node",
            "Test dispute description",
            vec![],
        )
        .unwrap();

    // Add evidence to the dispute
    governance
        .add_evidence(
            &dispute.id,
            "Additional evidence",
            vec![1, 2, 3],
        )
        .unwrap();

    // Verify the evidence was added
    let updated_dispute: Dispute = governance.storage
        .get_json(&format!("disputes/{}", dispute.id))
        .unwrap();
    
    assert_eq!(updated_dispute.evidence.len(), 1);
    assert_eq!(updated_dispute.evidence[0].description, "Additional evidence");
    assert_eq!(updated_dispute.status, DisputeStatus::UnderReview);
}

#[test]
fn test_resolve_dispute() {
    let (governance, _temp_dir) = setup_test();

    // Create a dispute first
    let dispute = governance
        .create_dispute(
            "test-federation",
            "test-transaction",
            "test-did:test:other-coop:other-node",
            "Test dispute description",
            vec![],
        )
        .unwrap();

    // Resolve the dispute
    governance
        .resolve_dispute(
            &dispute.id,
            "Dispute resolved in favor of complainant",
            vec!["Refund transaction".to_string()],
        )
        .unwrap();

    // Verify the dispute was resolved
    let updated_dispute: Dispute = governance.storage
        .get_json(&format!("disputes/{}", dispute.id))
        .unwrap();
    
    assert_eq!(updated_dispute.status, DisputeStatus::Resolved);
    assert!(updated_dispute.resolution.is_some());
    let resolution = updated_dispute.resolution.unwrap();
    assert_eq!(resolution.decision, "Dispute resolved in favor of complainant");
    assert_eq!(resolution.actions, vec!["Refund transaction"]);
}

#[test]
fn test_proposal_quorum_not_reached() {
    let (governance, _temp_dir) = setup_test();

    // Create a proposal with high quorum requirement
    let changes = serde_json::json!({
        "max_transaction_amount": 2000,
        "transaction_fee": 2
    });

    let proposal = governance
        .create_proposal(
            "test-federation",
            ProposalType::PolicyChange,
            "Test Proposal",
            "Test Description",
            1, // 1 second voting period
            5, // 5 votes required for quorum
            changes,
        )
        .unwrap();

    // Wait for voting period to end
    std::thread::sleep(std::time::Duration::from_secs(2));

    // Process the proposal
    let result = governance.process_proposal(&proposal.id);
    assert!(result.is_err());

    // Verify the proposal failed
    let updated_proposal: Proposal = governance.storage
        .get_json(&format!("proposals/{}", proposal.id))
        .unwrap();
    
    assert_eq!(updated_proposal.status, ProposalStatus::Failed);
}

#[test]
fn test_proposal_voting_period_expired() {
    let (governance, _temp_dir) = setup_test();

    // Create a proposal with short voting period
    let changes = serde_json::json!({
        "max_transaction_amount": 2000,
        "transaction_fee": 2
    });

    let proposal = governance
        .create_proposal(
            "test-federation",
            ProposalType::PolicyChange,
            "Test Proposal",
            "Test Description",
            1, // 1 second voting period
            1, // 1 vote required for quorum
            changes,
        )
        .unwrap();

    // Wait for voting period to end
    std::thread::sleep(std::time::Duration::from_secs(2));

    // Try to vote on the proposal
    let result = governance.vote(&proposal.id, true);
    assert!(result.is_err());
} 