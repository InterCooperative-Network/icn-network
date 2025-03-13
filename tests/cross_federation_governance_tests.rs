use std::sync::Arc;
use tempfile::tempdir;
use crate::cross_federation_governance::*;
use crate::identity::Identity;
use crate::storage::Storage;
use crate::crypto::CryptoUtils;
use crate::federation_governance::{Proposal, ProposalType, ProposalStatus};

fn setup_test() -> (CrossFederationGovernance, tempfile::TempDir) {
    let temp_dir = tempdir().unwrap();
    let storage = Storage::new(temp_dir.path().to_path_buf());
    let identity = Identity::new(
        "test-coop".to_string(),
        "test-node".to_string(),
        "test-did:test:test-coop:test-node".to_string(),
        storage.clone(),
    ).unwrap();
    let governance = CrossFederationGovernance::new(identity, storage);
    (governance, temp_dir)
}

#[test]
fn test_create_coordination() {
    let (governance, _temp_dir) = setup_test();

    let coordination = governance
        .create_coordination(
            CoordinationType::PolicyAlignment,
            "Test Coordination",
            "Test Description",
            3600, // 1 hour duration
            3,    // 3 federations required
        )
        .unwrap();

    assert_eq!(coordination.coordination_type, CoordinationType::PolicyAlignment);
    assert_eq!(coordination.title, "Test Coordination");
    assert_eq!(coordination.required_federations, 3);
    assert_eq!(coordination.status, CoordinationStatus::Draft);
    assert_eq!(coordination.participating_federations.len(), 1);
    assert!(coordination.proposals.is_empty());
    assert!(coordination.consensus.is_none());
}

#[test]
fn test_join_coordination() {
    let (governance, _temp_dir) = setup_test();

    // Create a coordination first
    let coordination = governance
        .create_coordination(
            CoordinationType::PolicyAlignment,
            "Test Coordination",
            "Test Description",
            3600,
            3,
        )
        .unwrap();

    // Join the coordination
    governance.join_coordination(&coordination.id).unwrap();

    // Verify the coordination was updated
    let updated_coordination: CrossFederationCoordination = governance.storage
        .get_json(&format!("cross_federation_coordinations/{}", coordination.id))
        .unwrap();
    
    assert_eq!(updated_coordination.status, CoordinationStatus::Active);
    assert_eq!(updated_coordination.participating_federations.len(), 1);
}

#[test]
fn test_submit_proposal() {
    let (governance, _temp_dir) = setup_test();

    // Create a coordination first
    let coordination = governance
        .create_coordination(
            CoordinationType::PolicyAlignment,
            "Test Coordination",
            "Test Description",
            3600,
            3,
        )
        .unwrap();

    // Create a proposal
    let proposal = Proposal {
        id: "test-proposal".to_string(),
        federation_id: "test-federation".to_string(),
        proposal_type: ProposalType::PolicyChange,
        title: "Test Proposal".to_string(),
        description: "Test Description".to_string(),
        created_by: governance.identity.did.clone(),
        created_at: 0,
        voting_start: 0,
        voting_end: 3600,
        quorum: 3,
        status: ProposalStatus::Active,
        votes: vec![],
        changes: serde_json::json!({
            "max_transaction_amount": 2000,
            "transaction_fee": 2
        }),
    };

    // Submit the proposal
    governance.submit_proposal(&coordination.id, proposal.clone()).unwrap();

    // Verify the proposal was added
    let updated_coordination: CrossFederationCoordination = governance.storage
        .get_json(&format!("cross_federation_coordinations/{}", coordination.id))
        .unwrap();
    
    assert_eq!(updated_coordination.proposals.len(), 1);
    assert_eq!(updated_coordination.proposals[0].id, proposal.id);
}

#[test]
fn test_reach_consensus() {
    let (governance, _temp_dir) = setup_test();

    // Create a coordination first
    let coordination = governance
        .create_coordination(
            CoordinationType::PolicyAlignment,
            "Test Coordination",
            "Test Description",
            3600,
            1, // Only 1 federation required for testing
        )
        .unwrap();

    // Create and submit a proposal
    let proposal = Proposal {
        id: "test-proposal".to_string(),
        federation_id: "test-federation".to_string(),
        proposal_type: ProposalType::PolicyChange,
        title: "Test Proposal".to_string(),
        description: "Test Description".to_string(),
        created_by: governance.identity.did.clone(),
        created_at: 0,
        voting_start: 0,
        voting_end: 3600,
        quorum: 3,
        status: ProposalStatus::Active,
        votes: vec![],
        changes: serde_json::json!({
            "max_transaction_amount": 2000,
            "transaction_fee": 2
        }),
    };

    governance.submit_proposal(&coordination.id, proposal.clone()).unwrap();

    // Reach consensus
    governance
        .reach_consensus(
            &coordination.id,
            vec![proposal.id.clone()],
            vec!["Implement policy changes".to_string()],
        )
        .unwrap();

    // Verify consensus was reached
    let updated_coordination: CrossFederationCoordination = governance.storage
        .get_json(&format!("cross_federation_coordinations/{}", coordination.id))
        .unwrap();
    
    assert_eq!(updated_coordination.status, CoordinationStatus::ConsensusReached);
    assert!(updated_coordination.consensus.is_some());
    let consensus = updated_coordination.consensus.unwrap();
    assert_eq!(consensus.agreed_proposals, vec![proposal.id]);
    assert_eq!(consensus.implementation_plan, vec!["Implement policy changes"]);
    assert_eq!(consensus.signatures.len(), 1);
}

#[test]
fn test_implement_consensus() {
    let (governance, _temp_dir) = setup_test();

    // Create a coordination first
    let coordination = governance
        .create_coordination(
            CoordinationType::PolicyAlignment,
            "Test Coordination",
            "Test Description",
            3600,
            1, // Only 1 federation required for testing
        )
        .unwrap();

    // Create and submit a proposal
    let proposal = Proposal {
        id: "test-proposal".to_string(),
        federation_id: "test-federation".to_string(),
        proposal_type: ProposalType::PolicyChange,
        title: "Test Proposal".to_string(),
        description: "Test Description".to_string(),
        created_by: governance.identity.did.clone(),
        created_at: 0,
        voting_start: 0,
        voting_end: 3600,
        quorum: 3,
        status: ProposalStatus::Active,
        votes: vec![],
        changes: serde_json::json!({
            "max_transaction_amount": 2000,
            "transaction_fee": 2
        }),
    };

    governance.submit_proposal(&coordination.id, proposal.clone()).unwrap();

    // Reach consensus
    governance
        .reach_consensus(
            &coordination.id,
            vec![proposal.id.clone()],
            vec!["Implement policy changes".to_string()],
        )
        .unwrap();

    // Implement consensus
    governance.implement_consensus(&coordination.id).unwrap();

    // Verify implementation
    let updated_coordination: CrossFederationCoordination = governance.storage
        .get_json(&format!("cross_federation_coordinations/{}", coordination.id))
        .unwrap();
    
    assert_eq!(updated_coordination.status, CoordinationStatus::Implemented);
}

#[test]
fn test_coordination_expired() {
    let (governance, _temp_dir) = setup_test();

    // Create a coordination with short duration
    let coordination = governance
        .create_coordination(
            CoordinationType::PolicyAlignment,
            "Test Coordination",
            "Test Description",
            1, // 1 second duration
            3,
        )
        .unwrap();

    // Wait for coordination to expire
    std::thread::sleep(std::time::Duration::from_secs(2));

    // Try to join the coordination
    let result = governance.join_coordination(&coordination.id);
    assert!(result.is_err());
}

#[test]
fn test_insufficient_federations() {
    let (governance, _temp_dir) = setup_test();

    // Create a coordination requiring multiple federations
    let coordination = governance
        .create_coordination(
            CoordinationType::PolicyAlignment,
            "Test Coordination",
            "Test Description",
            3600,
            3, // 3 federations required
        )
        .unwrap();

    // Try to reach consensus without enough federations
    let result = governance.reach_consensus(
        &coordination.id,
        vec![],
        vec![],
    );
    assert!(result.is_err());
} 