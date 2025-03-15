use icn_node::IcnNode;
use std::error::Error;
use tempfile::tempdir;
use serde_json::json;

#[tokio::test]
async fn test_node_governance_reputation_integration() -> Result<(), Box<dyn Error>> {
    // Create temporary directory for storage
    let temp_dir = tempdir()?;
    let storage_path = temp_dir.path().to_path_buf();
    
    // Create a test node
    let node = IcnNode::new(
        "test-coop".to_string(),
        "main-node".to_string(),
        storage_path.clone(),
        None, // Default network config
    )?;
    
    // Create a second test node
    let temp_dir2 = tempdir()?;
    let storage_path2 = temp_dir2.path().to_path_buf();
    let node2 = IcnNode::new(
        "test-coop".to_string(),
        "voter-node".to_string(),
        storage_path2.clone(),
        None, // Default network config
    )?;
    
    // Test federation ID
    let federation_id = "test-federation";
    
    // 1. Create a proposal through the node interface
    let proposal = node.create_proposal(
        federation_id,
        "policy_change", // ProposalType as string
        "Node Interface Test Proposal",
        "This is a test proposal created through the node interface",
        1, // 1 day voting duration
        2, // Quorum of 2 votes
        json!({
            "policy_change": "test_value"
        }),
    ).await?;
    
    // 2. Add a deliberation to the proposal
    let deliberation = node.add_deliberation(
        &proposal.id,
        "This is a thoughtful deliberation with detailed analysis of the proposal.",
        vec!["ref:documentation:123"], // Reference to documentation
    ).await?;
    
    // 3. Retrieve deliberations
    let deliberations = node.get_proposal_deliberations(&proposal.id).await?;
    assert_eq!(deliberations.len(), 1);
    assert_eq!(deliberations[0].comment, deliberation.comment);
    
    // 4. Vote on the proposal from both nodes
    node.vote_on_proposal(&proposal.id, true).await?; // Creator votes yes
    node2.vote_on_proposal(&proposal.id, false).await?; // Second node votes no
    
    // 5. Check governance score for the creator
    let creator_gov_score = node.get_governance_score(&node.identity.did).await?;
    
    // Creator should have created a proposal and voted
    assert!(creator_gov_score.overall_score > 0.0);
    assert_eq!(creator_gov_score.proposals_created, 1);
    assert_eq!(creator_gov_score.proposals_voted, 1);
    assert_eq!(creator_gov_score.deliberations_count, 1);
    
    // 6. Check governance score for the second node
    let voter_gov_score = node2.get_governance_score(&node2.identity.did).await?;
    
    // Voter should have only voted
    assert!(voter_gov_score.overall_score > 0.0);
    assert_eq!(voter_gov_score.proposals_created, 0);
    assert_eq!(voter_gov_score.proposals_voted, 1);
    assert_eq!(voter_gov_score.deliberations_count, 0);
    
    // 7. Get comprehensive trust score
    let trust_score = node.get_comprehensive_trust_score(&node.identity.did).await?;
    
    // The trust score should include governance component
    assert!(trust_score.overall_score > 0.0);
    assert!(trust_score.components.contains_key("GovernanceQuality"));
    
    // 8. Advanced: Test deliberation quality scoring
    // Add a detailed deliberation with multiple references
    let detailed_deliberation = node.add_deliberation(
        &proposal.id,
        "This is an extremely detailed and well-researched deliberation that analyzes the proposal from multiple perspectives. \
        It considers economic implications, governance structures, and technical feasibility. The analysis is supported by \
        quantitative data and references to previous proposals and external research. This deliberation also suggests \
        specific improvements to the proposal that could address potential concerns.",
        vec![
            "ref:previous-proposal:123".to_string(),
            "ref:research-paper:456".to_string(),
            "ref:economic-analysis:789".to_string(),
        ],
    ).await?;
    
    // Verify the detailed deliberation was recorded
    let updated_deliberations = node.get_proposal_deliberations(&proposal.id).await?;
    assert_eq!(updated_deliberations.len(), 2);
    
    // 9. Get updated governance score after detailed deliberation
    let updated_gov_score = node.get_governance_score(&node.identity.did).await?;
    
    // Score should be higher with better deliberation
    assert!(updated_gov_score.overall_score > creator_gov_score.overall_score);
    assert_eq!(updated_gov_score.deliberations_count, 2);
    
    // 10. Get updated comprehensive trust score
    let updated_trust_score = node.get_comprehensive_trust_score(&node.identity.did).await?;
    
    // The trust score should be higher with better deliberation
    assert!(updated_trust_score.overall_score > trust_score.overall_score);
    
    Ok(())
}

#[tokio::test]
async fn test_node_reputation_influences_credit() -> Result<(), Box<dyn Error>> {
    // Create temporary directory for storage
    let temp_dir = tempdir()?;
    let storage_path = temp_dir.path().to_path_buf();
    
    // Create a test node
    let node = IcnNode::new(
        "test-coop".to_string(),
        "credit-test-node".to_string(),
        storage_path.clone(),
        None, // Default network config
    )?;
    
    // Test federation ID
    let federation_id = "test-federation";
    
    // 1. Check initial credit limit
    let economic_system = node.economic.clone();
    let member_did = node.identity.did.clone();
    let initial_account = economic_system.get_or_create_member_account(&member_did)?;
    let initial_limit = initial_account.credit_limit;
    
    // 2. Create a proposal to build reputation
    let proposal = node.create_proposal(
        federation_id,
        "policy_change",
        "Credit Test Proposal",
        "Proposal to test reputation's effect on credit limits",
        1, // 1 day
        1, // Quorum of 1
        json!({ "test": true }),
    ).await?;
    
    // 3. Vote on own proposal
    node.vote_on_proposal(&proposal.id, true).await?;
    
    // 4. Add multiple high-quality deliberations
    for i in 1..=5 {
        node.add_deliberation(
            &proposal.id,
            format!("Detailed deliberation #{} with thorough analysis and considerations.", i),
            vec![format!("ref:document:{}", i)],
        ).await?;
    }
    
    // 5. Get a comprehensive trust score
    let trust_score = node.get_comprehensive_trust_score(&member_did).await?;
    assert!(trust_score.overall_score > 0.0);
    
    // 6. Check if credit limit has been adjusted based on reputation
    let updated_account = economic_system.get_or_create_member_account(&member_did)?;
    
    // If the economic module is properly using reputation for credit limits,
    // the limit should have increased
    assert!(updated_account.credit_limit > initial_limit);
    
    Ok(())
} 