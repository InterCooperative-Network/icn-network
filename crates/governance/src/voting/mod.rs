//! Voting management for governance

use std::collections::HashMap;
use serde::{Serialize, Deserialize};

use icn_common::types::{Value, DID};
use crate::{Proposal, ProposalStatus, Vote, VoteType, GovernanceResult, GovernanceError};

/// Create a new vote
pub fn create_vote(
    proposal_id: &str,
    voter: DID,
    vote_type: VoteType,
    weight: f64,
    metadata: HashMap<String, Value>,
) -> GovernanceResult<Vote> {
    // Generate vote ID
    let id = format!("vote-{}", uuid::Uuid::new_v4());
    
    // Get current timestamp
    let created_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    
    // Create the vote
    let vote = Vote {
        id,
        proposal_id: proposal_id.to_string(),
        voter_id: voter,
        vote_type,
        weight,
        created_at,
        signature: None,
        metadata,
    };
    
    Ok(vote)
}

/// Validate a vote
pub fn validate_vote(vote: &Vote, proposal: &Proposal) -> GovernanceResult<()> {
    // Check if proposal is in voting stage
    if proposal.status != ProposalStatus::Voting {
        return Err(GovernanceError::InvalidVote(
            format!("Proposal {} is not in voting stage", proposal.id)
        ));
    }
    
    // Check if voting period has ended
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    
    if now > proposal.voting_ends_at {
        return Err(GovernanceError::InvalidVote(
            format!("Voting period for proposal {} has ended", proposal.id)
        ));
    }
    
    // Additional validation based on vote type
    match &vote.vote_type {
        VoteType::RankedChoice(choices) => {
            // Validate ranked choice vote
            if choices.is_empty() {
                return Err(GovernanceError::InvalidVote("Ranked choice vote must have at least one choice".to_string()));
            }
            
            // Check for duplicates
            let mut seen = std::collections::HashSet::new();
            for choice in choices {
                if !seen.insert(choice) {
                    return Err(GovernanceError::InvalidVote("Ranked choice vote contains duplicate choices".to_string()));
                }
            }
        },
        VoteType::Custom(custom) => {
            // Validate custom vote type
            if custom.is_empty() {
                return Err(GovernanceError::InvalidVote("Custom vote type cannot be empty".to_string()));
            }
        },
        _ => {
            // No additional validation for standard vote types
        }
    }
    
    Ok(())
}

/// Calculate vote results for a proposal
pub fn calculate_vote_results(
    proposal: &Proposal,
    votes: &[Vote],
) -> GovernanceResult<VoteResults> {
    let mut total_weight = 0.0;
    let mut yes_weight = 0.0;
    let mut no_weight = 0.0;
    let mut abstain_weight = 0.0;
    let mut vote_count = votes.len();
    
    for vote in votes {
        match vote.vote_type {
            VoteType::Yes => yes_weight += vote.weight,
            VoteType::No => no_weight += vote.weight,
            VoteType::Abstain => abstain_weight += vote.weight,
            _ => {
                // For other vote types, count weight towards quorum but not decision
                vote_count -= 1;
            }
        }
        
        total_weight += vote.weight;
    }
    
    // Calculate quorum and approval percentages
    let quorum_reached = total_weight >= proposal.quorum;
    
    let approval_percentage = if (yes_weight + no_weight) > 0.0 {
        yes_weight / (yes_weight + no_weight)
    } else {
        0.0
    };
    
    let approved = quorum_reached && approval_percentage >= proposal.approval_threshold;
    
    // Determine the final status
    let status = if !quorum_reached {
        ProposalStatus::Rejected
    } else if approved {
        ProposalStatus::Passed
    } else {
        ProposalStatus::Rejected
    };
    
    Ok(VoteResults {
        proposal_id: proposal.id.clone(),
        vote_count,
        total_weight,
        yes_weight,
        no_weight,
        abstain_weight,
        quorum_reached,
        approval_percentage,
        approved,
        status,
    })
}

/// Results of voting on a proposal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteResults {
    /// Proposal ID
    pub proposal_id: String,
    /// Number of votes cast
    pub vote_count: usize,
    /// Total weight of all votes
    pub total_weight: f64,
    /// Total weight of yes votes
    pub yes_weight: f64,
    /// Total weight of no votes
    pub no_weight: f64,
    /// Total weight of abstain votes
    pub abstain_weight: f64,
    /// Whether quorum was reached
    pub quorum_reached: bool,
    /// Percentage of approval (yes votes / (yes + no votes))
    pub approval_percentage: f64,
    /// Whether the proposal is approved
    pub approved: bool,
    /// Final status of the proposal
    pub status: ProposalStatus,
} 