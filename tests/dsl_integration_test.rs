/// Integration test for DSL functionality
///
/// This test verifies that the DSL can be parsed, interpreted, and integrated
/// with the governance system.

use anyhow::Result;
use icn_core::init_tracing;
use icn_governance::{
    ProposalManager, ProposalStatus,
    dsl::{GovernanceDslManager, DslEvent, parse},
};
use std::sync::Arc;

#[tokio::test]
async fn test_dsl_integration() -> Result<()> {
    // Initialize tracing
    init_tracing();
    
    // Create a proposal manager
    let proposal_manager = Arc::new(ProposalManager::new().await?);
    
    // Create a DSL manager
    let mut dsl_manager = GovernanceDslManager::new(Arc::clone(&proposal_manager)).await;
    
    // Simple DSL script
    let script = r#"
        proposal "TestProposal" {
            title: "Test Proposal"
            description: "A test proposal for integration testing"
            
            voting {
                method: "majority"
                threshold: 51%
                quorum: 30%
            }
            
            on_approve {
                log("Proposal approved")
            }
            
            on_reject {
                log("Proposal rejected")
            }
        }
    "#;
    
    // Try parsing the script
    let program = parse(script)?;
    assert!(!program.statements.is_empty(), "Parsed program should have statements");
    
    // Execute the script in a separate task
    let dsl_task = tokio::spawn(async move {
        // Execute the script
        dsl_manager.execute_script(script).await?;
        
        // Process events for a short time
        let timeout = tokio::time::timeout(
            tokio::time::Duration::from_secs(1),
            dsl_manager.start_event_processing()
        ).await;
        
        // Ignore timeout error - we expect this because event processing runs until the channel closes
        match timeout {
            Ok(result) => result,
            Err(_) => Ok(()),
        }
    });
    
    // Wait a moment for the script to be processed
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    
    // Check if a proposal was created
    let proposals = proposal_manager.list_proposals().await?;
    assert!(!proposals.is_empty(), "At least one proposal should have been created");
    
    // Verify proposal properties
    if let Some(proposal) = proposals.first() {
        assert_eq!(proposal.title, "Test Proposal");
        assert_eq!(proposal.status, ProposalStatus::Active);
        
        // Cast votes
        proposal_manager.cast_vote(&proposal.id, "alice", true).await?;
        proposal_manager.cast_vote(&proposal.id, "bob", true).await?;
        proposal_manager.cast_vote(&proposal.id, "carol", false).await?;
        
        // Check vote tally
        let tally = proposal_manager.get_vote_tally(&proposal.id).await?;
        assert_eq!(tally.yes_votes, 2);
        assert_eq!(tally.no_votes, 1);
        
        // Execute the proposal
        proposal_manager.mark_proposal_executed(&proposal.id).await?;
        
        // Verify the proposal was executed
        let updated_proposal = proposal_manager.get_proposal(&proposal.id).await?;
        assert_eq!(updated_proposal.status, ProposalStatus::Executed);
    }
    
    // Wait for DSL processing to complete
    let _ = dsl_task.await;
    
    Ok(())
} 