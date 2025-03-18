/// Governance Integration Module
///
/// This module provides integration between the DSL system and the governance system.
/// It allows DSL scripts to interact with the governance system, create proposals,
/// cast votes, and execute approved proposals.

use crate::dsl::parser::{Ast, AstNode, ProposalNode};
use crate::governance::{GovernanceService, ProposalStatus, ProposalType};
use anyhow::{Result, Context};
use std::sync::Arc;
use std::collections::HashMap;

/// Governance Integration
pub struct GovernanceIntegration {
    /// Governance service
    governance_service: Arc<GovernanceService>,
}

impl GovernanceIntegration {
    /// Create a new governance integration
    pub fn new(governance_service: Arc<GovernanceService>) -> Self {
        Self { governance_service }
    }
    
    /// Process governance-related AST nodes
    pub async fn process_ast(&self, ast: &Ast, federation: &str) -> Result<()> {
        for node in &ast.nodes {
            if let AstNode::Proposal(proposal) = node {
                self.create_proposal(proposal, federation).await?;
            }
        }
        
        Ok(())
    }
    
    /// Create a proposal from a DSL proposal node
    async fn create_proposal(&self, proposal: &ProposalNode, federation: &str) -> Result<()> {
        // Convert DSL proposal to governance proposal
        self.governance_service.create_proposal(
            &proposal.title,
            &proposal.description,
            ProposalType::Policy, // Default type, could be extracted from DSL
            federation,
            "dsl_system", // Default proposer, could be extracted from DSL
            51, // Default quorum, could be extracted from DSL
            51, // Default approval, could be extracted from DSL
            None, // content_file
        ).await.context("Failed to create proposal from DSL")?;
        
        Ok(())
    }
    
    /// Cast a vote on a proposal
    pub async fn cast_vote(
        &self,
        proposal_id: &str,
        voter_id: &str,
        vote: &str,
        weight: f64,
        federation: &str,
    ) -> Result<()> {
        self.governance_service.vote(
            proposal_id,
            voter_id,
            vote,
            None, // comment
            weight,
            federation,
        ).await.context("Failed to cast vote from DSL")?;
        
        Ok(())
    }
    
    /// Execute a proposal
    pub async fn execute_proposal(&self, proposal_id: &str, federation: &str) -> Result<()> {
        self.governance_service.execute_proposal(
            proposal_id,
            federation,
        ).await.context("Failed to execute proposal from DSL")?;
        
        Ok(())
    }
    
    /// Get all proposals
    pub async fn get_proposals(&self, federation: &str) -> Result<HashMap<String, ProposalDetails>> {
        // In a real implementation, this would fetch proposals from the governance system
        // For now, we'll return an empty map
        Ok(HashMap::new())
    }
}

/// Proposal details returned from the governance system
pub struct ProposalDetails {
    /// Proposal ID
    pub id: String,
    /// Proposal title
    pub title: String,
    /// Proposal description
    pub description: String,
    /// Proposal status
    pub status: String,
    /// Votes
    pub votes: HashMap<String, VoteDetails>,
}

/// Vote details
pub struct VoteDetails {
    /// Voter ID
    pub voter_id: String,
    /// Vote (yes, no, abstain)
    pub vote: String,
    /// Vote weight
    pub weight: f64,
    /// Optional comment
    pub comment: Option<String>,
} 