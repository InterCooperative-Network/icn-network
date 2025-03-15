use icn_node::crypto::CryptoUtils;
use icn_node::identity::Identity;
use icn_node::reputation::{ReputationSystem, AttestationType};
use icn_node::storage::Storage;
use icn_node::federation_governance::{
    FederationGovernance, ProposalType, ProposalStatus,
    Deliberation, GovernanceParticipationScore
};

use std::sync::Arc;
use std::error::Error;
use std::time::{SystemTime, UNIX_EPOCH, Duration};
use tempfile::tempdir;
use serde_json::json;

// Helper function to create a test environment
async fn setup_test_environment() -> Result<(
    Arc<Identity>, 
    Arc<Storage>, 
    Arc<CryptoUtils>, 
    Arc<ReputationSystem>, 
    Arc<FederationGovernance>
), Box<dyn Error>> {
    // Create temporary directory for storage
    let temp_dir = tempdir()?;
    let storage = Arc::new(Storage::new(temp_dir.path().to_path_buf())?);
    
    // Create identity
    let identity = Arc::new(Identity::new(
        "test-coop".to_string(),
        "test-node".to_string(),
        "did:icn:test-coop:test-node".to_string(),
        storage.clone(),
    )?);
    
    // Create crypto utils
    let crypto = Arc::new(CryptoUtils::new());
    
    // Create reputation system
    let reputation = Arc::new(ReputationSystem::new(
        identity.clone(),
        storage.clone(),
        crypto.clone(),
    ));
    
    // Create federation governance with reputation
    let mut governance = FederationGovernance::new(
        identity.clone(),
        storage.clone(),
    );
    governance.set_reputation_system(reputation.clone());
    let governance = Arc::new(governance);
    
    Ok((identity, storage, crypto, reputation, governance))
}

#[tokio::test]
async fn test_governance_reputation_integration() -> Result<(), Box<dyn Error>> {
    // Set up test environment
    let (identity, storage, crypto, reputation, governance) = setup_test_environment().await?;
    
    // Test federation ID
    let federation_id = "test-federation";
    
    // 1. Create a proposal
    let proposal = governance.create_proposal(
        federation_id,
        ProposalType::PolicyChange,
        "Test Proposal",
        "This is a test proposal for governance reputation integration",
        1, // 1 day voting period
        2, // Quorum of 2 votes
        json!({
            "policy_change": "test_value"
        }),
    )?;
    
    // 2. Verify reputation was created for proposal creation
    let creator_did = identity.did.clone();
    let creator_score = reputation.calculate_trust_score(&creator_did)?;
    
    // Creator should have positive reputation from creating proposal
    assert!(creator_score.overall_score > 0.0);
    assert!(creator_score.components.contains_key("GovernanceQuality"));
    
    // 3. Create a second identity for voting
    let voter_identity = Arc::new(Identity::new(
        "test-coop".to_string(),
        "voter-node".to_string(),
        "did:icn:test-coop:voter-node".to_string(),
        storage.clone(),
    )?);
    
    // 4. Add a deliberation to the proposal
    let deliberation = governance.add_deliberation(
        &proposal.id,
        "This is a thoughtful comment about the proposal with detailed considerations.",
        vec![], // No references
    ).await?;
    
    // Verify the deliberation was recorded
    let deliberations = governance.get_deliberations(&proposal.id)?;
    assert_eq!(deliberations.len(), 1);
    assert_eq!(deliberations[0].comment, deliberation.comment);
    
    // 5. Vote on the proposal
    governance.vote(&proposal.id, true).await?;
    
    // 6. Create a new governance with the voter identity
    let mut voter_governance = FederationGovernance::new(
        voter_identity.clone(),
        storage.clone(),
    );
    voter_governance.set_reputation_system(reputation.clone());
    
    // 7. Second participant votes
    voter_governance.vote(&proposal.id, false).await?;
    
    // 8. Process the proposal to update reputation
    // First, we need to force the voting period to end
    let mut proposal_updated: serde_json::Value = storage.load_json(&format!("proposals/{}", proposal.id))?;
    if let Some(voting_end) = proposal_updated["voting_end"].as_u64() {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        proposal_updated["voting_end"] = json!(now - 10); // Set voting period to have ended 10 seconds ago
    }
    storage.store_json(&format!("proposals/{}", proposal.id), &proposal_updated)?;
    
    // Process the proposal
    governance.process_proposal(&proposal.id).await?;
    
    // 9. Calculate governance scores
    let creator_gov_score = governance.calculate_governance_score(&creator_did).await?;
    let voter_gov_score = governance.calculate_governance_score(&voter_identity.did).await?;
    
    // Verify scores
    assert!(creator_gov_score.overall_score > 0.0);
    assert!(voter_gov_score.overall_score > 0.0);
    
    // Creator should have created proposals and voted
    assert_eq!(creator_gov_score.proposals_created, 1);
    assert_eq!(creator_gov_score.proposals_voted, 1);
    
    // Voter should have only voted
    assert_eq!(voter_gov_score.proposals_created, 0);
    assert_eq!(voter_gov_score.proposals_voted, 1);
    
    // 10. Verify updated reputation after proposal completion
    let creator_final_score = reputation.calculate_trust_score(&creator_did)?;
    let voter_final_score = reputation.calculate_trust_score(&voter_identity.did)?;
    
    // Scores should be higher than initial scores
    assert!(creator_final_score.overall_score > creator_score.overall_score);
    assert!(voter_final_score.overall_score > 0.0);
    
    // Components related to governance should exist
    assert!(creator_final_score.components.contains_key("GovernanceQuality"));
    assert!(voter_final_score.components.contains_key("GovernanceQuality"));
    
    Ok(())
}

#[tokio::test]
async fn test_deliberation_reputation() -> Result<(), Box<dyn Error>> {
    // Set up test environment
    let (identity, storage, crypto, reputation, governance) = setup_test_environment().await?;
    
    // Test federation ID
    let federation_id = "test-federation";
    
    // 1. Create a proposal
    let proposal = governance.create_proposal(
        federation_id,
        ProposalType::PolicyChange,
        "Deliberation Test Proposal",
        "Testing how deliberation affects reputation scores",
        1, // 1 day voting period
        1, // Quorum of 1 vote
        json!({ "test": true }),
    )?;
    
    // 2. Add a simple deliberation
    let simple_comment = "Simple comment.";
    governance.add_deliberation(
        &proposal.id,
        simple_comment,
        vec![],
    ).await?;
    
    // 3. Add a detailed deliberation with references
    let detailed_comment = "This is a very detailed and thoughtful analysis of the proposal. \
        It considers multiple aspects and provides evidence-based reasoning for why this proposal \
        should be accepted. The analysis takes into account economic impacts, governance implications, \
        and long-term sustainability considerations for the cooperative network.";
    
    let reference1 = "ref:previous-proposal:123";
    let reference2 = "ref:research-document:456";
    governance.add_deliberation(
        &proposal.id,
        detailed_comment,
        vec![reference1.to_string(), reference2.to_string()],
    ).await?;
    
    // 4. Get trust score
    let member_did = identity.did.clone();
    let trust_score = reputation.calculate_trust_score(&member_did)?;
    
    // 5. Verify reputation components reflect deliberation quality
    assert!(trust_score.components.contains_key("GovernanceQuality"));
    
    // 6. Vote on the proposal to complete governance cycle
    governance.vote(&proposal.id, true).await?;
    
    // 7. Process the proposal to update reputation
    // First, we need to force the voting period to end
    let mut proposal_updated: serde_json::Value = storage.load_json(&format!("proposals/{}", proposal.id))?;
    if let Some(voting_end) = proposal_updated["voting_end"].as_u64() {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        proposal_updated["voting_end"] = json!(now - 10); // Set voting period to have ended 10 seconds ago
    }
    storage.store_json(&format!("proposals/{}", proposal.id), &proposal_updated)?;
    
    // Process the proposal
    governance.process_proposal(&proposal.id).await?;
    
    // 8. Calculate final governance score
    let gov_score = governance.calculate_governance_score(&member_did).await?;
    
    // 9. Verify deliberation count
    assert_eq!(gov_score.deliberations_count, 2);
    
    // 10. Get final trust score
    let final_trust_score = reputation.calculate_trust_score(&member_did)?;
    
    // Should have higher score than initial
    assert!(final_trust_score.overall_score > trust_score.overall_score);
    
    Ok(())
} 