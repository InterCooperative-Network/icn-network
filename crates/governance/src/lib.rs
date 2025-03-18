//! Governance module for ICN
//!
//! This module provides decentralized governance capabilities for the
//! InterCooperative Network, including proposal creation, voting, and execution.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::fmt;

use async_trait::async_trait;
use tokio::sync::RwLock;
use thiserror::Error;
use serde::{Serialize, Deserialize};

use icn_core::{
    crypto::{Signature, Hash, identity::NodeId, sha256},
    storage::{Storage, StorageResult, StorageError, JsonStorage},
    utils::timestamp_secs,
};

use icn_identity::IdentityService;

// Define the types we need
pub type IdentityResult<T> = Result<T, GovernanceError>;
pub type IdentityError = GovernanceError;  // We'll map identity errors to governance errors
pub type Identity = NodeId;  // For simplicity, identity is just a NodeId for now

/// Error types for governance operations
#[derive(Error, Debug)]
pub enum GovernanceError {
    /// Error with the identity system
    #[error("Identity error: {0}")]
    IdentityError(String),
    
    /// Error with storage
    #[error("Storage error: {0}")]
    StorageError(#[from] StorageError),
    
    /// Error with reputation
    #[error("Reputation error: {0}")]
    ReputationError(String),
    
    /// Invalid proposal
    #[error("Invalid proposal: {0}")]
    InvalidProposal(String),
    
    /// Invalid vote
    #[error("Invalid vote: {0}")]
    InvalidVote(String),
    
    /// Proposal not found
    #[error("Proposal not found: {0}")]
    ProposalNotFound(String),
    
    /// Vote not found
    #[error("Vote not found: {0}")]
    VoteNotFound(String),
    
    /// Permission denied
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    
    /// IO error
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),
}

/// Result type for governance operations
pub type GovernanceResult<T> = Result<T, GovernanceError>;

/// Status of a proposal
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProposalStatus {
    /// Draft status, not yet submitted for voting
    Draft,
    /// Open for voting
    Open,
    /// Voting has closed, waiting for processing
    Closed,
    /// Proposal has been approved
    Approved,
    /// Proposal has been rejected
    Rejected,
    /// Proposal has been executed
    Executed,
    /// Proposal has failed execution
    Failed,
    /// Proposal has been cancelled
    Cancelled,
}

impl Default for ProposalStatus {
    fn default() -> Self {
        Self::Draft
    }
}

/// Types of proposals
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProposalType {
    /// Change in configuration
    ConfigChange,
    /// Add a new member
    AddMember,
    /// Remove a member
    RemoveMember,
    /// Upgrade of software
    SoftwareUpgrade,
    /// Allocation of resources
    ResourceAllocation,
    /// Generic proposal
    Generic,
    /// Custom proposal type
    Custom(String),
}

impl Default for ProposalType {
    fn default() -> Self {
        Self::Generic
    }
}

/// A vote on a proposal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vote {
    /// The proposal ID this vote is for
    pub proposal_id: String,
    /// The voter's identity
    pub voter: NodeId,
    /// The vote choice (true = yes, false = no)
    pub approve: bool,
    /// Optional comment with the vote
    pub comment: Option<String>,
    /// The voting weight (if weighted voting is used)
    pub weight: Option<f64>,
    /// When the vote was cast
    pub timestamp: u64,
    /// The signature from the voter
    pub signature: Signature,
}

impl Vote {
    /// Create a new unsigned vote
    pub fn new(
        proposal_id: String,
        voter: NodeId,
        approve: bool,
        comment: Option<String>,
        weight: Option<f64>,
    ) -> Self {
        Self {
            proposal_id,
            voter,
            approve,
            comment,
            weight,
            timestamp: timestamp_secs(),
            signature: Signature(Vec::new()), // Placeholder, will be set when signed
        }
    }
    
    /// Get the bytes to sign for this vote
    pub fn bytes_to_sign(&self) -> Vec<u8> {
        // Serialize the vote data without the signature
        let serializable = VoteData {
            proposal_id: self.proposal_id.clone(),
            voter: self.voter.clone(),
            approve: self.approve,
            comment: self.comment.clone(),
            weight: self.weight,
            timestamp: self.timestamp,
        };
        
        serde_json::to_vec(&serializable).unwrap_or_default()
    }
}

/// Serializable vote data for signing
#[derive(Serialize, Deserialize)]
struct VoteData {
    /// The proposal ID this vote is for
    pub proposal_id: String,
    /// The voter's identity
    pub voter: NodeId,
    /// The vote choice (true = yes, false = no)
    pub approve: bool,
    /// Optional comment with the vote
    pub comment: Option<String>,
    /// The voting weight (if weighted voting is used)
    pub weight: Option<f64>,
    /// When the vote was cast
    pub timestamp: u64,
}

/// A proposal for governance decisions
#[derive(Clone, Serialize, Deserialize)]
pub struct Proposal {
    /// Unique identifier for this proposal
    pub id: String,
    /// The title of the proposal
    pub title: String,
    /// The detailed description of the proposal
    pub description: String,
    /// The type of proposal
    pub proposal_type: ProposalType,
    /// The identity that submitted the proposal
    pub proposer: NodeId,
    /// The current status of the proposal
    pub status: ProposalStatus,
    /// When the proposal was created
    pub created_at: u64,
    /// When voting opens
    pub voting_starts_at: u64,
    /// When voting closes
    pub voting_ends_at: u64,
    /// When the proposal was processed
    pub processed_at: Option<u64>,
    /// The result of the vote (if processed)
    pub result: Option<String>,
    /// Additional attributes for the proposal
    pub attributes: HashMap<String, String>,
    /// The signature from the proposer
    pub signature: Signature,
}

impl Proposal {
    /// Create a new unsigned proposal
    pub fn new(
        title: String,
        description: String,
        proposal_type: ProposalType,
        proposer: NodeId,
        voting_starts_at: u64,
        voting_ends_at: u64,
        attributes: HashMap<String, String>,
    ) -> Self {
        let created_at = timestamp_secs();
        let id = format!("proposal-{}-{}", proposer, created_at);
        
        Self {
            id,
            title,
            description,
            proposal_type,
            proposer,
            status: ProposalStatus::Draft,
            created_at,
            voting_starts_at,
            voting_ends_at,
            processed_at: None,
            result: None,
            attributes,
            signature: Signature(Vec::new()), // Placeholder, will be set when signed
        }
    }
    
    /// Get the bytes to sign for this proposal
    pub fn bytes_to_sign(&self) -> Vec<u8> {
        // Serialize the proposal data without the signature
        let serializable = ProposalData {
            id: self.id.clone(),
            title: self.title.clone(),
            description: self.description.clone(),
            proposal_type: self.proposal_type.clone(),
            proposer: self.proposer.clone(),
            status: self.status.clone(),
            created_at: self.created_at,
            voting_starts_at: self.voting_starts_at,
            voting_ends_at: self.voting_ends_at,
            processed_at: self.processed_at,
            result: self.result.clone(),
            attributes: self.attributes.clone(),
        };
        
        serde_json::to_vec(&serializable).unwrap_or_default()
    }
    
    /// Check if the proposal is currently open for voting
    pub fn is_open_for_voting(&self) -> bool {
        if self.status != ProposalStatus::Open {
            return false;
        }
        
        let now = timestamp_secs();
        now >= self.voting_starts_at && now <= self.voting_ends_at
    }
    
    /// Check if the voting period is over
    pub fn is_voting_closed(&self) -> bool {
        let now = timestamp_secs();
        now > self.voting_ends_at
    }
}

impl fmt::Debug for Proposal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Proposal {{ id: {}, title: {}, type: {:?}, status: {:?}, proposer: {} }}",
            self.id, self.title, self.proposal_type, self.status, self.proposer)
    }
}

/// Serializable proposal data for signing
#[derive(Serialize, Deserialize)]
struct ProposalData {
    /// Unique identifier for this proposal
    pub id: String,
    /// The title of the proposal
    pub title: String,
    /// The detailed description of the proposal
    pub description: String,
    /// The type of proposal
    pub proposal_type: ProposalType,
    /// The identity that submitted the proposal
    pub proposer: NodeId,
    /// The current status of the proposal
    pub status: ProposalStatus,
    /// When the proposal was created
    pub created_at: u64,
    /// When voting opens
    pub voting_starts_at: u64,
    /// When voting ends
    pub voting_ends_at: u64,
    /// When the proposal was processed
    pub processed_at: Option<u64>,
    /// The result of the vote (if processed)
    pub result: Option<String>,
    /// Additional attributes for the proposal
    pub attributes: HashMap<String, String>,
}

/// Configuration for the governance system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceConfig {
    /// Minimum reputation required to create a proposal
    pub min_proposal_reputation: f64,
    /// Minimum reputation required to vote
    pub min_voting_reputation: f64,
    /// Default voting period in seconds
    pub default_voting_period: u64,
    /// Reputation gained for creating a proposal
    pub proposal_creation_reputation: f64,
    /// Reputation gained for voting on a proposal
    pub voting_reputation: f64,
    /// Whether to use reputation-weighted voting
    pub use_weighted_voting: bool,
    /// Quorum as a percentage of total possible votes (0.0 to 1.0)
    pub quorum_percentage: f64,
    /// Percentage of yes votes required to approve (0.0 to 1.0)
    pub approval_percentage: f64,
    /// Custom governance rules
    pub custom_rules: HashMap<String, String>,
}

impl Default for GovernanceConfig {
    fn default() -> Self {
        Self {
            min_proposal_reputation: 0.5,  // Require moderate reputation to propose
            min_voting_reputation: 0.2,    // Low barrier to vote
            default_voting_period: 86400,  // 24 hours
            proposal_creation_reputation: 0.05, // Small reputation boost for creating proposals
            voting_reputation: 0.02,      // Small reputation boost for voting
            use_weighted_voting: true,    // Use reputation-weighted voting by default
            quorum_percentage: 0.25,      // Require 25% participation for validity
            approval_percentage: 0.6,     // Require 60% approval to pass
            custom_rules: HashMap::new(),
        }
    }
}

/// A trait for governance operations
#[async_trait]
pub trait Governance: Send + Sync {
    /// Get the governance configuration
    async fn get_config(&self) -> GovernanceResult<GovernanceConfig>;
    
    /// Set the governance configuration
    async fn set_config(&self, config: GovernanceConfig) -> GovernanceResult<()>;
    
    /// Create a new proposal
    async fn create_proposal(
        &self,
        title: String,
        description: String,
        proposal_type: ProposalType,
        voting_period: Option<u64>,
        attributes: HashMap<String, String>,
    ) -> GovernanceResult<Proposal>;
    
    /// Get a proposal by ID
    async fn get_proposal(&self, id: &str) -> GovernanceResult<Option<Proposal>>;
    
    /// List all proposals
    async fn list_proposals(&self) -> GovernanceResult<Vec<Proposal>>;
    
    /// Cast a vote on a proposal
    async fn vote(
        &self,
        proposal_id: &str,
        approve: bool,
        comment: Option<String>,
    ) -> GovernanceResult<Vote>;
    
    /// Get votes for a proposal
    async fn get_votes(&self, proposal_id: &str) -> GovernanceResult<Vec<Vote>>;
    
    /// Process a proposal after voting is complete
    async fn process_proposal(&self, proposal_id: &str) -> GovernanceResult<ProposalStatus>;
    
    /// Cancel a proposal (only allowed by the proposer or admins)
    async fn cancel_proposal(&self, proposal_id: &str) -> GovernanceResult<()>;
}

pub mod execution;
pub mod voting;
pub mod manager;
pub mod dao;
pub mod dsl;

// Re-exports
pub use manager::GovernanceManager;
pub use voting::{VotingScheme, SimpleVoting, WeightedVoting};
pub use execution::ProposalExecutor;

// ICN Governance crate

// Governance system for ICN, including proposals and voting.

/// Governance types and utilities
pub mod governance {
    /// A simple proposal struct
    #[derive(Debug, Clone)]
    pub struct Proposal {
        /// The identifier for this proposal
        pub id: String,
        /// The title of the proposal
        pub title: String,
        /// The description of the proposal
        pub description: String,
    }

    impl Proposal {
        /// Create a new proposal
        pub fn new(id: &str, title: &str, description: &str) -> Self {
            Self {
                id: id.to_string(),
                title: title.to_string(),
                description: description.to_string(),
            }
        }
    }
}

/// ProposalManager for the governance system
pub struct ProposalManager;

/// Vote tally
#[derive(Debug)]
pub struct VoteTally {
    /// Number of yes votes
    pub yes_votes: usize,
    /// Number of no votes
    pub no_votes: usize,
    /// Number of abstentions
    pub abstentions: usize,
    /// Total number of votes
    pub total_votes: usize,
}

impl ProposalManager {
    /// Create a new proposal manager
    pub async fn new() -> anyhow::Result<Self> {
        Ok(Self {})
    }
    
    /// Create a simple proposal
    pub async fn create_proposal(&self, id: &str, title: &str, description: &str) -> anyhow::Result<()> {
        // Just a stub implementation for now
        Ok(())
    }
    
    /// Cast a vote on a proposal
    pub async fn cast_vote(&self, proposal_id: &str, voter_id: &str, approve: bool) -> anyhow::Result<()> {
        // Just a stub implementation for now
        Ok(())
    }
    
    /// Get the vote tally for a proposal
    pub async fn get_vote_tally(&self, proposal_id: &str) -> anyhow::Result<VoteTally> {
        // Just a stub implementation for now
        Ok(VoteTally {
            yes_votes: 0,
            no_votes: 0,
            abstentions: 0,
            total_votes: 0,
        })
    }
    
    /// Mark a proposal as executed
    pub async fn mark_proposal_executed(&self, proposal_id: &str) -> anyhow::Result<()> {
        // Just a stub implementation for now
        Ok(())
    }
    
    /// Get a proposal by ID
    pub async fn get_proposal(&self, proposal_id: &str) -> anyhow::Result<Proposal> {
        // Just a stub implementation for now
        Ok(Proposal {
            id: proposal_id.to_string(),
            title: "Stub Proposal".to_string(),
            description: "This is a stub proposal".to_string(),
            status: ProposalStatus::Active,
        })
    }
    
    /// List all proposals
    pub async fn list_proposals(&self) -> anyhow::Result<Vec<Proposal>> {
        // Just a stub implementation for now
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_proposal_creation() {
        let proposal = governance::Proposal::new("test-id", "Test Proposal", "A test proposal");
        assert_eq!(proposal.id, "test-id");
        assert_eq!(proposal.title, "Test Proposal");
        assert_eq!(proposal.description, "A test proposal");
    }
} 