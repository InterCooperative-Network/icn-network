//! Governance module for ICN federations
//!
//! This module handles decentralized governance operations for ICN federations,
//! including proposal creation, voting, and execution of governance decisions.
//!
//! ## Features
//!
//! - **Democratic voting** with configurable voting systems
//! - **Proposal management** for policy changes and member management
//! - **Deliberation systems** for structured discussion
//! - **Reputation tracking** for governance participants
//! - **Smart contract-based execution** of approved proposals

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::fs;
use tracing::{debug, info, warn};

/// Types of governance proposals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProposalType {
    /// Change to federation policy
    PolicyChange,
    /// Add a new member to the federation
    MemberAddition,
    /// Remove an existing member from the federation
    MemberRemoval,
    /// Adjust resource allocation for the federation
    ResourceAllocation,
    /// Dispute resolution between members
    DisputeResolution,
    /// Modify federation configuration
    ConfigChange,
}

/// Status of a governance proposal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProposalStatus {
    /// Proposal is in draft stage
    Draft,
    /// Proposal is in deliberation phase
    Deliberation,
    /// Proposal is open for voting
    Voting,
    /// Proposal has been approved
    Approved,
    /// Proposal has been rejected
    Rejected,
    /// Proposal has been executed
    Executed,
    /// Proposal has been canceled
    Canceled,
}

/// Vote cast on a proposal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Vote {
    /// Vote in favor of the proposal
    Yes,
    /// Vote against the proposal
    No,
    /// Abstain from voting
    Abstain,
}

/// Governance proposal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    /// Unique identifier for the proposal
    pub id: String,
    /// Title of the proposal
    pub title: String,
    /// Detailed description of the proposal
    pub description: String,
    /// Type of proposal
    pub proposal_type: ProposalType,
    /// Current status of the proposal
    pub status: ProposalStatus,
    /// Member who created the proposal
    pub proposer: String,
    /// When the proposal was created
    pub created_at: u64,
    /// When the proposal was last updated
    pub updated_at: u64,
    /// When voting begins
    pub voting_starts_at: Option<u64>,
    /// When voting ends
    pub voting_ends_at: Option<u64>,
    /// Votes cast on the proposal
    pub votes: Vec<MemberVote>,
    /// Minimum quorum percentage required (0-100)
    pub quorum_percentage: u8,
    /// Minimum approval percentage required (0-100)
    pub approval_percentage: u8,
    /// Structured data representing the proposal content
    pub content: serde_json::Value,
}

/// Vote cast by a member
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberVote {
    /// Member ID who cast the vote
    pub member_id: String,
    /// The vote cast
    pub vote: Vote,
    /// When the vote was cast
    pub timestamp: u64,
    /// Optional comment with the vote
    pub comment: Option<String>,
    /// Voting weight of this member
    pub weight: f64,
}

/// Federation governance service
pub struct GovernanceService {
    /// Federation ID
    federation_id: String,
    /// Base path for governance data
    base_path: std::path::PathBuf,
    /// Loaded proposals
    proposals: Vec<Proposal>,
}

impl GovernanceService {
    /// Create a new governance service for a federation
    pub async fn new(federation_id: &str, base_path: impl AsRef<Path>) -> Result<Self> {
        let base_path = base_path.as_ref().to_path_buf();
        let governance_path = base_path.join("governance").join(federation_id);
        
        // Create governance directory if it doesn't exist
        if !governance_path.exists() {
            fs::create_dir_all(&governance_path).await?;
        }
        
        // Load existing proposals if any
        let proposals_path = governance_path.join("proposals.json");
        let proposals = if proposals_path.exists() {
            let data = fs::read(&proposals_path).await?;
            serde_json::from_slice(&data)?
        } else {
            Vec::new()
        };
        
        Ok(Self {
            federation_id: federation_id.to_string(),
            base_path,
            proposals,
        })
    }
    
    /// Create a new governance proposal
    pub async fn create_proposal(
        &mut self,
        title: &str,
        description: &str,
        proposal_type: ProposalType,
        proposer: &str,
        content: serde_json::Value,
        quorum_percentage: u8,
        approval_percentage: u8,
    ) -> Result<String> {
        // Generate unique ID
        let id = uuid::Uuid::new_v4().to_string();
        
        // Get current timestamp
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();
        
        // Create proposal
        let proposal = Proposal {
            id: id.clone(),
            title: title.to_string(),
            description: description.to_string(),
            proposal_type,
            status: ProposalStatus::Draft,
            proposer: proposer.to_string(),
            created_at: timestamp,
            updated_at: timestamp,
            voting_starts_at: None,
            voting_ends_at: None,
            votes: Vec::new(),
            quorum_percentage,
            approval_percentage,
            content,
        };
        
        // Add to proposals list
        self.proposals.push(proposal);
        
        // Save proposals
        self.save_proposals().await?;
        
        info!("Created proposal {} in federation {}", id, self.federation_id);
        Ok(id)
    }
    
    /// Get all proposals in the federation
    pub fn get_proposals(&self) -> &[Proposal] {
        &self.proposals
    }
    
    /// Get a specific proposal by ID
    pub fn get_proposal(&self, id: &str) -> Option<&Proposal> {
        self.proposals.iter().find(|p| p.id == id)
    }
    
    /// Update proposal status
    pub async fn update_proposal_status(&mut self, id: &str, status: ProposalStatus) -> Result<()> {
        let proposal = self.proposals.iter_mut()
            .find(|p| p.id == id)
            .ok_or_else(|| anyhow!("Proposal not found"))?;
        
        proposal.status = status;
        proposal.updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();
        
        self.save_proposals().await?;
        
        info!("Updated proposal {} status to {:?}", id, status);
        Ok(())
    }
    
    /// Cast a vote on a proposal
    pub async fn cast_vote(
        &mut self,
        proposal_id: &str,
        member_id: &str,
        vote: Vote,
        comment: Option<String>,
        weight: f64,
    ) -> Result<()> {
        let proposal = self.proposals.iter_mut()
            .find(|p| p.id == proposal_id)
            .ok_or_else(|| anyhow!("Proposal not found"))?;
        
        // Check if proposal is in voting stage
        match proposal.status {
            ProposalStatus::Voting => {},
            _ => return Err(anyhow!("Proposal is not in voting stage")),
        }
        
        // Check if member has already voted
        if proposal.votes.iter().any(|v| v.member_id == member_id) {
            return Err(anyhow!("Member has already voted on this proposal"));
        }
        
        // Get current timestamp
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();
        
        // Create vote
        let member_vote = MemberVote {
            member_id: member_id.to_string(),
            vote,
            timestamp,
            comment,
            weight,
        };
        
        // Add vote to proposal
        proposal.votes.push(member_vote);
        proposal.updated_at = timestamp;
        
        // Check if voting should end (all members voted or deadline reached)
        if let Some(ends_at) = proposal.voting_ends_at {
            if timestamp >= ends_at {
                self.finalize_voting(proposal_id).await?;
            }
        }
        
        self.save_proposals().await?;
        
        info!("Recorded vote from {} on proposal {}", member_id, proposal_id);
        Ok(())
    }
    
    /// Start voting period for a proposal
    pub async fn start_voting(
        &mut self,
        proposal_id: &str,
        duration_seconds: u64,
    ) -> Result<()> {
        let proposal = self.proposals.iter_mut()
            .find(|p| p.id == proposal_id)
            .ok_or_else(|| anyhow!("Proposal not found"))?;
        
        // Check if proposal is in draft or deliberation stage
        match proposal.status {
            ProposalStatus::Draft | ProposalStatus::Deliberation => {},
            _ => return Err(anyhow!("Proposal cannot be moved to voting stage")),
        }
        
        // Get current timestamp
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();
        
        proposal.status = ProposalStatus::Voting;
        proposal.voting_starts_at = Some(timestamp);
        proposal.voting_ends_at = Some(timestamp + duration_seconds);
        proposal.updated_at = timestamp;
        
        self.save_proposals().await?;
        
        info!("Started voting for proposal {} (ends in {} seconds)", proposal_id, duration_seconds);
        Ok(())
    }
    
    /// Finalize voting on a proposal
    pub async fn finalize_voting(&mut self, proposal_id: &str) -> Result<()> {
        let proposal = self.proposals.iter_mut()
            .find(|p| p.id == proposal_id)
            .ok_or_else(|| anyhow!("Proposal not found"))?;
        
        // Check if proposal is in voting stage
        if !matches!(proposal.status, ProposalStatus::Voting) {
            return Err(anyhow!("Proposal is not in voting stage"));
        }
        
        // Calculate results
        let total_weight: f64 = proposal.votes.iter().map(|v| v.weight).sum();
        let yes_weight: f64 = proposal.votes.iter()
            .filter(|v| matches!(v.vote, Vote::Yes))
            .map(|v| v.weight)
            .sum();
        
        // Calculate percentages
        let participation_percentage = if total_weight > 0.0 { (total_weight * 100.0) } else { 0.0 };
        let approval_percentage = if total_weight > 0.0 { (yes_weight / total_weight) * 100.0 } else { 0.0 };
        
        // Determine result
        let quorum_reached = participation_percentage as u8 >= proposal.quorum_percentage;
        let approved = approval_percentage as u8 >= proposal.approval_percentage && quorum_reached;
        
        proposal.status = if approved {
            ProposalStatus::Approved
        } else {
            ProposalStatus::Rejected
        };
        
        proposal.updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();
        
        self.save_proposals().await?;
        
        info!(
            "Finalized voting for proposal {}: {}",
            proposal_id,
            if approved { "APPROVED" } else { "REJECTED" }
        );
        
        Ok(())
    }
    
    /// Execute an approved proposal
    pub async fn execute_proposal(&mut self, proposal_id: &str) -> Result<()> {
        let proposal = self.proposals.iter_mut()
            .find(|p| p.id == proposal_id)
            .ok_or_else(|| anyhow!("Proposal not found"))?;
        
        // Check if proposal is approved
        if !matches!(proposal.status, ProposalStatus::Approved) {
            return Err(anyhow!("Proposal is not approved and cannot be executed"));
        }
        
        // TODO: Implement actual execution for different proposal types
        
        // Mark as executed
        proposal.status = ProposalStatus::Executed;
        proposal.updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();
        
        self.save_proposals().await?;
        
        info!("Executed proposal {}", proposal_id);
        Ok(())
    }
    
    // Helper to save proposals to disk
    async fn save_proposals(&self) -> Result<()> {
        let governance_path = self.base_path.join("governance").join(&self.federation_id);
        let proposals_path = governance_path.join("proposals.json");
        
        // Create parent directory if it doesn't exist
        if let Some(parent) = proposals_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).await?;
            }
        }
        
        // Serialize and save
        let data = serde_json::to_vec(&self.proposals)?;
        fs::write(&proposals_path, &data).await?;
        
        Ok(())
    }
} 