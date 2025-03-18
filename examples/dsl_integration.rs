/// DSL Integration Example
///
/// This example demonstrates how to use the DSL system with the governance
/// system to create and execute proposals.

use anyhow::Result;
use icn_core::init_tracing;
use icn_governance::{
    ProposalManager, Proposal, ProposalStatus,
    dsl::{GovernanceDslManager, DslEvent},
};
use std::sync::Arc;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    init_tracing();
    
    // Create a simple in-memory proposal manager
    let proposal_manager = Arc::new(ProposalManager::new().await?);
    
    // Create the DSL manager with the proposal manager
    let mut dsl_manager = GovernanceDslManager::new(Arc::clone(&proposal_manager)).await;
    
    // Load and execute a simple DSL script
    let script = r#"
        proposal "EducationBudget" {
            title: "Fund Education Program"
            description: "Allocate 500 credits to the Education Program for workshop supplies and speaker fees."
            
            voting {
                method: "ranked_choice"
                threshold: 60%
                quorum: 51%
                duration: "7 days"
            }
            
            on_approve {
                transaction {
                    from: "treasury"
                    to: "education_program"
                    amount: 500
                    asset: "credits"
                }
                
                log("Budget allocation for Education Program approved and executed.")
            }
            
            on_reject {
                log("Budget allocation for Education Program was rejected.")
            }
        }
    "#;
    
    println!("Executing DSL script");
    
    // You can run this in a separate task to avoid blocking
    let handle = tokio::spawn(async move {
        match dsl_manager.execute_script(script).await {
            Ok(_) => println!("Script executed successfully"),
            Err(e) => eprintln!("Script execution failed: {}", e),
        }
        
        // Start processing events
        match dsl_manager.start_event_processing().await {
            Ok(_) => println!("Event processing completed"),
            Err(e) => eprintln!("Event processing failed: {}", e),
        }
    });
    
    // Wait a moment for the script to execute
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    
    // Check for created proposals
    let proposals = proposal_manager.list_proposals().await?;
    println!("\nProposals after execution:");
    for proposal in proposals {
        println!("- ID: {}", proposal.id);
        println!("  Title: {}", proposal.title);
        println!("  Description: {}", proposal.description);
        println!("  Status: {:?}", proposal.status);
        println!();
    }
    
    // Simulate voting on a proposal
    if let Some(proposal) = proposal_manager.list_proposals().await?.first() {
        println!("Casting votes on proposal: {}", proposal.id);
        
        // Simulate multiple votes
        let votes = vec![
            ("alice", true),
            ("bob", true),
            ("carol", false),
            ("dave", true),
            ("eve", true),
        ];
        
        for (voter, vote) in votes {
            match proposal_manager.cast_vote(&proposal.id, voter, vote).await {
                Ok(_) => println!("  Vote cast by {}: {}", voter, if vote { "Yes" } else { "No" }),
                Err(e) => eprintln!("  Failed to cast vote by {}: {}", voter, e),
            }
        }
        
        // Check the vote tally
        let tally = proposal_manager.get_vote_tally(&proposal.id).await?;
        println!("\nVote tally for proposal {}:", proposal.id);
        println!("  Yes votes: {}", tally.yes_votes);
        println!("  No votes: {}", tally.no_votes);
        println!("  Abstentions: {}", tally.abstentions);
        println!("  Total votes: {}", tally.total_votes);
        
        // Execute the proposal if it passed
        if tally.yes_votes > tally.no_votes {
            println!("\nProposal passed, executing...");
            proposal_manager.mark_proposal_executed(&proposal.id).await?;
        } else {
            println!("\nProposal rejected.");
        }
    }
    
    // Check the final state of proposals
    let proposals = proposal_manager.list_proposals().await?;
    println!("\nFinal proposal states:");
    for proposal in proposals {
        println!("- ID: {}", proposal.id);
        println!("  Title: {}", proposal.title);
        println!("  Status: {:?}", proposal.status);
        println!();
    }
    
    // Wait for the DSL processing to complete
    let _ = handle.await;
    
    Ok(())
} 