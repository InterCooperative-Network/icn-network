//! Proposal management for governance

use std::collections::HashMap;
use serde::{Serialize, Deserialize};

use icn_common::types::{Value, DID};
use crate::{Proposal, ProposalType, ProposalStatus, GovernanceResult, GovernanceError};

/// Create a new proposal
pub fn create_proposal(
    title: &str,
    description: &str,
    creator: DID,
    federation_id: &str,
    proposal_type: ProposalType,
    voting_period: u64,
    quorum: f64,
    approval_threshold: f64,
    metadata: HashMap<String, Value>,
) -> GovernanceResult<Proposal> {
    // Validate proposal parameters
    if title.is_empty() {
        return Err(GovernanceError::InvalidProposal("Title cannot be empty".to_string()));
    }
    
    if description.is_empty() {
        return Err(GovernanceError::InvalidProposal("Description cannot be empty".to_string()));
    }
    
    if federation_id.is_empty() {
        return Err(GovernanceError::InvalidProposal("Federation ID cannot be empty".to_string()));
    }
    
    if quorum <= 0.0 || quorum > 1.0 {
        return Err(GovernanceError::InvalidProposal("Quorum must be between 0 and 1".to_string()));
    }
    
    if approval_threshold <= 0.0 || approval_threshold > 1.0 {
        return Err(GovernanceError::InvalidProposal("Approval threshold must be between 0 and 1".to_string()));
    }
    
    // Generate proposal ID
    let id = format!("proposal-{}", uuid::Uuid::new_v4());
    
    // Get current timestamp
    let created_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    
    // Calculate voting end timestamp
    let voting_ends_at = created_at + voting_period;
    
    // Create the proposal
    let proposal = Proposal {
        id,
        title: title.to_string(),
        description: description.to_string(),
        creator,
        federation_id: federation_id.to_string(),
        proposal_type,
        status: ProposalStatus::Draft,
        created_at,
        voting_ends_at,
        execution_deadline: Some(voting_ends_at + 86400), // 24 hours after voting ends
        quorum,
        approval_threshold,
        metadata,
    };
    
    Ok(proposal)
}

/// Validate a proposal before submission
pub fn validate_proposal(proposal: &Proposal) -> GovernanceResult<()> {
    // Basic validation
    if proposal.title.is_empty() {
        return Err(GovernanceError::InvalidProposal("Title cannot be empty".to_string()));
    }
    
    if proposal.description.is_empty() {
        return Err(GovernanceError::InvalidProposal("Description cannot be empty".to_string()));
    }
    
    if proposal.federation_id.is_empty() {
        return Err(GovernanceError::InvalidProposal("Federation ID cannot be empty".to_string()));
    }
    
    if proposal.quorum <= 0.0 || proposal.quorum > 1.0 {
        return Err(GovernanceError::InvalidProposal("Quorum must be between 0 and 1".to_string()));
    }
    
    if proposal.approval_threshold <= 0.0 || proposal.approval_threshold > 1.0 {
        return Err(GovernanceError::InvalidProposal("Approval threshold must be between 0 and 1".to_string()));
    }
    
    // Additional validation based on proposal type
    match &proposal.proposal_type {
        ProposalType::GovernanceChange => {
            // Validate governance change proposal
            if !proposal.metadata.contains_key("param_name") {
                return Err(GovernanceError::InvalidProposal("Governance change proposals must specify the parameter name".to_string()));
            }
            
            if !proposal.metadata.contains_key("param_value") {
                return Err(GovernanceError::InvalidProposal("Governance change proposals must specify the parameter value".to_string()));
            }
        },
        ProposalType::ResourceAllocation => {
            // Validate resource allocation proposal
            if !proposal.metadata.contains_key("resource_type") {
                return Err(GovernanceError::InvalidProposal("Resource allocation proposals must specify the resource type".to_string()));
            }
            
            if !proposal.metadata.contains_key("amount") {
                return Err(GovernanceError::InvalidProposal("Resource allocation proposals must specify the amount".to_string()));
            }
            
            if !proposal.metadata.contains_key("recipient") {
                return Err(GovernanceError::InvalidProposal("Resource allocation proposals must specify the recipient".to_string()));
            }
        },
        _ => {
            // No additional validation for other proposal types
        }
    }
    
    Ok(())
}

/// Check if a proposal is eligible for execution
pub fn is_executable(proposal: &Proposal) -> bool {
    match proposal.status {
        ProposalStatus::Passed => {
            // Check if execution deadline has passed
            if let Some(deadline) = proposal.execution_deadline {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                
                now <= deadline
            } else {
                true // No deadline, can be executed
            }
        },
        _ => false, // Not in passed status
    }
} 