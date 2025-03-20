//! Execution of governance proposals

use std::collections::HashMap;
use std::sync::Arc;

use icn_common::types::{Value, DID};
use crate::{Proposal, ProposalStatus, GovernanceResult, GovernanceError};
use crate::integrations::GovernanceVMIntegration;

/// Execute a governance proposal
pub async fn execute_proposal(
    proposal: &mut Proposal,
    vm_integration: Arc<GovernanceVMIntegration>,
    executor: DID,
) -> GovernanceResult<()> {
    // Check if proposal is in the right state
    if proposal.status != ProposalStatus::Passed {
        return Err(GovernanceError::InvalidProposal(
            format!("Proposal {} is not in passed state", proposal.id)
        ));
    }
    
    // Check execution deadline if set
    if let Some(deadline) = proposal.execution_deadline {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        if now > deadline {
            proposal.status = ProposalStatus::Failed;
            return Err(GovernanceError::InvalidProposal(
                format!("Execution deadline for proposal {} has passed", proposal.id)
            ));
        }
    }
    
    // Execute the proposal through VM integration
    match vm_integration.execute_proposal(&proposal.id).await {
        Ok(_) => {
            // Update proposal status
            proposal.status = ProposalStatus::Executed;
            
            // Add execution metadata
            proposal.metadata.insert(
                "executed_by".to_string(),
                Value::String(executor.as_str().to_string()),
            );
            
            proposal.metadata.insert(
                "executed_at".to_string(),
                Value::Int(
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs() as i64
                ),
            );
            
            Ok(())
        },
        Err(e) => {
            // Mark proposal as failed
            proposal.status = ProposalStatus::Failed;
            
            // Add failure metadata
            proposal.metadata.insert(
                "execution_error".to_string(),
                Value::String(e.to_string()),
            );
            
            Err(e)
        }
    }
}

/// Cancel a governance proposal
pub fn cancel_proposal(
    proposal: &mut Proposal,
    canceler: DID,
    reason: &str,
) -> GovernanceResult<()> {
    // Check if proposal can be canceled
    match proposal.status {
        ProposalStatus::Draft | ProposalStatus::Voting => {
            // Check if canceler is the creator or has admin rights
            // In a real implementation, we would check admin roles
            if proposal.creator.as_str() != canceler.as_str() {
                return Err(GovernanceError::Unauthorized(
                    format!("Only the creator can cancel proposal {}", proposal.id)
                ));
            }
            
            // Update proposal status
            proposal.status = ProposalStatus::Cancelled;
            
            // Add cancellation metadata
            proposal.metadata.insert(
                "cancelled_by".to_string(),
                Value::String(canceler.as_str().to_string()),
            );
            
            proposal.metadata.insert(
                "cancelled_at".to_string(),
                Value::Int(
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs() as i64
                ),
            );
            
            proposal.metadata.insert(
                "cancellation_reason".to_string(),
                Value::String(reason.to_string()),
            );
            
            Ok(())
        },
        _ => {
            Err(GovernanceError::InvalidProposal(
                format!("Proposal {} in state {:?} cannot be cancelled", proposal.id, proposal.status)
            ))
        }
    }
} 