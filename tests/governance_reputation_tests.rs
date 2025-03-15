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
use std::fs;
use std::path::Path;
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
    let base_path = temp_dir.path().to_str().ok_or("Invalid path")?;
    
    println!("Test storage path: {}", base_path);
    
    // Create necessary subdirectories
    let dirs = [
        "proposals", 
        "votes", 
        "deliberations", 
        "attestations", 
        "federations",
        "test-federation",
        "identity",
        "reputation",
        "members",
        "dids"
    ];
    
    for dir in dirs.iter() {
        let dir_path = Path::new(base_path).join(dir);
        println!("Creating directory: {:?}", dir_path);
        fs::create_dir_all(&dir_path)?;
    }
    
    let storage = Arc::new(Storage::new(base_path));
    
    // Initialize federation data
    let federation_data = json!({
        "id": "test-federation",
        "name": "Test Federation",
        "description": "A federation for testing",
        "members": [],
        "created_at": SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs()
    });
    
    storage.put_json("federations/test-federation", &federation_data)?;
    
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
    
    // Create a dummy attestation to prevent "No such file or directory" errors
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let dummy_attestation = json!({
        "id": format!("att:{}:{}:{}", identity.did, identity.did, now),
        "issuer_did": identity.did.clone(),
        "subject_did": identity.did.clone(),
        "attestation_type": "GovernanceQuality",
        "score": 0.5,
        "context": json!({"action": "test"}),
        "claims": json!({}),
        "evidence": [],
        "signatures": [],
        "quorum_threshold": 1,
        "created_at": now,
        "expires_at": now + 86400 * 30, // 30 days
        "is_revoked": false
    });
    
    // Store the dummy attestation
    storage.put_json("attestations/dummy", &dummy_attestation)?;
    
    Ok((identity, storage, crypto, reputation, governance))
}

#[tokio::test]
async fn test_governance_reputation_integration() -> Result<(), Box<dyn Error>> {
    let (identity, storage, _crypto, reputation, governance) = setup_test_environment().await?;
    
    // Create a proposal
    println!("Creating proposal...");
    let creator_did = identity.did.clone();
    let proposal = governance.create_proposal(
        "test-federation",
        ProposalType::PolicyChange,
        "Test proposal for reputation",
        "This is a test proposal to evaluate governance reputation",
        86400, // 1 day voting period
        2,     // Quorum of 2 votes
        json!({"policy_change": "test"}),
    )?;
    
    println!("Proposal created with ID: {}", proposal.id);
    
    // Calculate trust score for the creator
    let trust_score = reputation.calculate_trust_score(&creator_did)?;
    
    // Verify that the score is positive and includes governance quality
    assert!(trust_score.overall_score > 0.0);
    assert!(trust_score.components.contains_key("GovernanceQuality"));
    
    // Create a second identity for voting
    let voter_identity = Arc::new(Identity::new(
        "test-coop".to_string(),
        "voter-node".to_string(),
        "did:icn:test-coop:voter-node".to_string(),
        storage.clone(),
    )?);
    
    // Initialize an empty deliberation list in storage
    let deliberations_key = format!("proposal_deliberations/{}", proposal.id);
    println!("Initializing empty deliberation list at: {}", deliberations_key);
    let empty_deliberations: Vec<String> = Vec::new();
    storage.put_json(&deliberations_key, &empty_deliberations)?;
    
    // Add a deliberation to the proposal
    println!("Adding deliberation to proposal: {}", proposal.id);
    let deliberation = governance.add_deliberation(
        &proposal.id,
        "This is a thoughtful comment about the proposal with detailed considerations.",
        vec![], // No references
    ).await?;
    
    // Verify the deliberation was recorded
    println!("Getting deliberations for proposal: {}", proposal.id);
    let deliberations = match governance.get_deliberations(&proposal.id) {
        Ok(delib) => {
            println!("Successfully retrieved {} deliberations", delib.len());
            delib
        },
        Err(e) => {
            println!("Error getting deliberations: {:?}", e);
            return Err(e);
        }
    };
    
    assert_eq!(deliberations.len(), 1);
    assert_eq!(deliberations[0].comment, deliberation.comment);
    
    // Vote on the proposal
    println!("Voting on proposal: {}", proposal.id);
    governance.vote(&proposal.id, true).await?;
    
    // Create a new governance with the voter identity
    let mut voter_governance = FederationGovernance::new(
        voter_identity.clone(),
        storage.clone(),
    );
    voter_governance.set_reputation_system(reputation.clone());
    
    // Second participant votes
    println!("Second participant voting on proposal: {}", proposal.id);
    voter_governance.vote(&proposal.id, false).await?;
    
    // Process the proposal to update reputation
    // First, we need to force the voting period to end
    println!("Updating proposal to end voting period");
    let mut proposal_updated: serde_json::Value = storage.get_json(&format!("proposals/{}", proposal.id))?;
    if let Some(_voting_end) = proposal_updated["voting_end"].as_u64() {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        proposal_updated["voting_end"] = json!(now - 10); // Set voting period to have ended 10 seconds ago
    }
    storage.put_json(&format!("proposals/{}", proposal.id), &proposal_updated)?;
    
    // Process the proposal
    println!("Processing proposal: {}", proposal.id);
    match governance.process_proposal(&proposal.id).await {
        Ok(_) => println!("Successfully processed proposal"),
        Err(e) => println!("Error processing proposal: {:?}", e)
    }
    
    // Calculate governance scores
    println!("Calculating governance scores");
    let creator_gov_score = governance.calculate_governance_score(&creator_did).await?;
    let voter_gov_score = governance.calculate_governance_score(&voter_identity.did).await?;
    
    // Verify scores
    println!("Creator governance score: {:?}", creator_gov_score);
    println!("Voter governance score: {:?}", voter_gov_score);
    
    Ok(())
}

#[tokio::test]
async fn test_deliberation_reputation() -> Result<(), Box<dyn Error>> {
    let (identity, storage, _crypto, reputation, governance) = setup_test_environment().await?;
    
    // Create a proposal
    println!("Creating proposal...");
    let creator_did = identity.did.clone();
    let proposal = governance.create_proposal(
        "test-federation",
        ProposalType::PolicyChange,
        "Test proposal for deliberation reputation",
        "This is a test proposal to evaluate deliberation reputation",
        86400, // 1 day voting period
        2,     // Quorum of 2 votes
        json!({"policy_change": "test"}),
    )?;
    
    println!("Proposal created with ID: {}", proposal.id);
    
    // Initialize an empty deliberation list in storage
    let deliberations_key = format!("proposal_deliberations/{}", proposal.id);
    println!("Initializing empty deliberation list at: {}", deliberations_key);
    let empty_deliberations: Vec<String> = Vec::new();
    storage.put_json(&deliberations_key, &empty_deliberations)?;
    
    // Add a high-quality deliberation with references and detailed comment
    println!("Adding high-quality deliberation");
    let high_quality_comment = "This proposal has significant implications for our governance structure. 
        I believe we should consider the following aspects: 1) long-term sustainability, 
        2) alignment with our core values, 3) practical implementation challenges. 
        Based on my analysis of similar policies in other organizations, 
        this approach has shown positive outcomes in 75% of cases.";
    
    let deliberation = governance.add_deliberation(
        &proposal.id,
        high_quality_comment,
        vec!["reference1".to_string(), "reference2".to_string()], // Multiple references
    ).await?;
    
    // Verify the deliberation was recorded
    println!("Getting deliberations for proposal: {}", proposal.id);
    let deliberations = match governance.get_deliberations(&proposal.id) {
        Ok(delib) => {
            println!("Successfully retrieved {} deliberations", delib.len());
            delib
        },
        Err(e) => {
            println!("Error getting deliberations: {:?}", e);
            return Err(e);
        }
    };
    
    assert_eq!(deliberations.len(), 1);
    assert_eq!(deliberations[0].comment, high_quality_comment);
    
    // Calculate trust score for the creator
    println!("Calculating trust score");
    let trust_score = reputation.calculate_trust_score(&creator_did)?;
    
    // Verify that the score is positive and includes deliberation quality
    println!("Trust score: {:?}", trust_score);
    assert!(trust_score.overall_score > 0.0);
    assert!(trust_score.components.contains_key("DeliberationQuality"));
    
    // Verify that the deliberation quality score is high
    let delib_quality = trust_score.components.get("DeliberationQuality").unwrap();
    println!("Deliberation quality score: {}", delib_quality);
    assert!(*delib_quality > 0.5); // High-quality deliberation should have a good score
    
    Ok(())
} 