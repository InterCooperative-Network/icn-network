//! Governance module for ICN
//!
//! This module provides decentralized governance capabilities for the
//! InterCooperative Network, including proposal creation, voting, and execution.

pub mod proposals;
pub mod voting;
pub mod execution;
pub mod integrations;

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use thiserror::Error;
use serde::{Serialize, Deserialize};
use async_trait::async_trait;

use icn_common::types::{Value, DID};
use icn_common::error::{CommonError, Result as CommonResult};

pub use integrations::GovernanceVMIntegration;

/// Error types for governance operations
#[derive(Error, Debug)]
pub enum GovernanceError {
    #[error("Invalid proposal: {0}")]
    InvalidProposal(String),
    
    #[error("Invalid vote: {0}")]
    InvalidVote(String),
    
    #[error("Unauthorized: {0}")]
    Unauthorized(String),
    
    #[error("Not found: {0}")]
    NotFound(String),
    
    #[error("VM execution error: {0}")]
    VMExecutionError(String),
    
    #[error("Storage error: {0}")]
    StorageError(String),
    
    #[error("Internal error: {0}")]
    InternalError(String),
}

/// Result type for governance operations
pub type GovernanceResult<T> = std::result::Result<T, GovernanceError>;

// Convert from GovernanceError to CommonError
impl From<GovernanceError> for CommonError {
    fn from(err: GovernanceError) -> Self {
        CommonError::Governance(err.to_string())
    }
}

/// A governance proposal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    /// Unique identifier
    pub id: String,
    /// Proposal title
    pub title: String,
    /// Detailed description
    pub description: String,
    /// Creator/author of the proposal
    pub creator: DID,
    /// Federation this proposal belongs to
    pub federation_id: String,
    /// Proposal type
    pub proposal_type: ProposalType,
    /// Proposal status
    pub status: ProposalStatus,
    /// Creation timestamp
    pub created_at: u64,
    /// Voting end timestamp
    pub voting_ends_at: u64,
    /// Execution deadline
    pub execution_deadline: Option<u64>,
    /// Quorum percentage required
    pub quorum: f64,
    /// Approval threshold percentage
    pub approval_threshold: f64,
    /// Proposal metadata
    pub metadata: HashMap<String, Value>,
}

/// Type of governance proposal
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProposalType {
    /// Change to governance parameters
    GovernanceChange,
    /// Change to economic parameters
    EconomicChange,
    /// Resource allocation
    ResourceAllocation,
    /// Role assignment
    RoleAssignment,
    /// Federation membership
    MembershipChange,
    /// General proposal
    General,
    /// Custom proposal type
    Custom(String),
}

/// Status of a governance proposal
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProposalStatus {
    /// Proposal is in draft stage
    Draft,
    /// Proposal is open for voting
    Voting,
    /// Proposal has passed and is awaiting execution
    Passed,
    /// Proposal has been rejected
    Rejected,
    /// Proposal has been executed
    Executed,
    /// Proposal has failed during execution
    Failed,
    /// Proposal has been cancelled
    Cancelled,
}

/// A vote on a governance proposal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vote {
    /// Unique identifier
    pub id: String,
    /// Related proposal ID
    pub proposal_id: String,
    /// Voter's DID
    pub voter_id: DID,
    /// Vote value
    pub vote_type: VoteType,
    /// Voting power
    pub weight: f64,
    /// Creation timestamp
    pub created_at: u64,
    /// Vote signature
    pub signature: Option<String>,
    /// Vote metadata
    pub metadata: HashMap<String, Value>,
}

/// Type of vote
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VoteType {
    /// Yes vote
    Yes,
    /// No vote
    No,
    /// Abstain vote
    Abstain,
    /// Ranked choice voting
    RankedChoice(Vec<String>),
    /// Custom vote type
    Custom(String),
}

/// Governance trait defining core governance operations
#[async_trait]
pub trait Governance: Send + Sync {
    /// Create a new proposal
    async fn create_proposal(&self, proposal: Proposal) -> GovernanceResult<Proposal>;
    
    /// Cast a vote on a proposal
    async fn cast_vote(&self, vote: Vote) -> GovernanceResult<Vote>;
    
    /// Get a proposal by ID
    async fn get_proposal(&self, proposal_id: &str) -> GovernanceResult<Proposal>;
    
    /// Get votes for a proposal
    async fn get_votes(&self, proposal_id: &str) -> GovernanceResult<Vec<Vote>>;
    
    /// Execute a proposal
    async fn execute_proposal(&self, proposal_id: &str) -> GovernanceResult<()>;
    
    /// Cancel a proposal
    async fn cancel_proposal(&self, proposal_id: &str, reason: &str) -> GovernanceResult<()>;
}

pub mod manager;
pub mod dao;
pub mod dsl;
pub mod federation;

// Re-exports
pub use manager::GovernanceManager;
pub use voting::{VotingScheme, SimpleVoting, WeightedVoting};
pub use execution::ProposalExecutor;
pub use federation::{
    FederationGovernance,
    FederationGovernanceError,
    CoordinationType,
    CoordinationStatus,
    CrossFederationCoordination,
    Consensus,
    ConsensusSignature,
    GovernanceEvidence,
    Dispute,
    DisputeResolution,
};

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
            status: ProposalStatus::Open,
            created_at: icn_core::utils::timestamp_secs(),
            proposer: NodeId::from_string("stub-proposer"),
            proposal_type: ProposalType::Generic,
            voting_starts_at: icn_core::utils::timestamp_secs(),
            voting_ends_at: icn_core::utils::timestamp_secs() + 86400,
            processed_at: None,
            result: None,
            attributes: HashMap::new(),
            signature: Signature(Vec::new()),
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

// Implement Reputation-related types for use in governance
pub mod reputation {
    use std::collections::HashMap;
    use async_trait::async_trait;
    use icn_core::crypto::identity::NodeId;
    use crate::GovernanceError;

    /// Result type for reputation operations
    pub type ReputationResult<T> = Result<T, GovernanceError>;
    
    /// Types of evidence
    #[derive(Debug, Clone)]
    pub enum EvidenceType {
        /// A successful transaction or interaction
        SuccessfulTransaction,
        /// A failed transaction or interaction
        FailedTransaction,
        /// Positive feedback from another identity
        PositiveFeedback,
        /// Negative feedback from another identity
        NegativeFeedback,
        /// Validation of some work or contribution
        Validation,
        /// Attestation from a trusted identity
        Attestation,
        /// Governance participation (voting, proposals)
        GovernanceParticipation,
        /// Participation in voting
        Voting,
        /// Creation of a proposal
        ProposalCreation,
        /// Execution of a proposal
        ProposalExecution,
        /// A custom evidence type
        Custom(String),
    }
    
    /// Evidence about an identity
    #[derive(Debug, Clone)]
    pub struct Evidence {
        /// Unique identifier for this evidence
        pub id: String,
        /// The identity that submitted the evidence
        pub submitter: NodeId,
        /// The identity the evidence is about
        pub subject: NodeId,
        /// The type of evidence
        pub evidence_type: EvidenceType,
        /// A description of the evidence
        pub description: String,
        /// The weight of this evidence (-1.0 to 1.0)
        pub weight: f64,
        /// When the evidence was created
        pub created_at: u64,
    }
    
    impl Evidence {
        /// Create new evidence
        pub fn new(
            submitter: NodeId,
            subject: NodeId,
            evidence_type: EvidenceType,
            description: String,
            weight: f64,
        ) -> Self {
            Self {
                id: format!("ev-{}", rand::random::<u64>()),
                submitter,
                subject,
                evidence_type,
                description,
                weight,
                created_at: icn_core::utils::timestamp_secs(),
            }
        }
    }
    
    /// Reputation score for an identity
    #[derive(Debug, Clone)]
    pub struct ReputationScore {
        /// The identity this score is for
        pub identity_id: NodeId,
        /// The overall score (0.0 to 1.0)
        pub score: f64,
        /// The number of positive evidence items
        pub positive_count: u32,
        /// The number of negative evidence items
        pub negative_count: u32,
        /// The total number of evidence items
        pub total_count: u32,
        /// Scores by category
        pub category_scores: HashMap<String, f64>,
        /// Last updated timestamp
        pub updated_at: u64,
    }
    
    impl ReputationScore {
        /// Create a new reputation score
        pub fn new(identity_id: NodeId) -> Self {
            Self {
                identity_id,
                score: 0.5,  // Default neutral score
                positive_count: 0,
                negative_count: 0,
                total_count: 0,
                category_scores: HashMap::new(),
                updated_at: icn_core::utils::timestamp_secs(),
            }
        }
    }
    
    /// Trait for reputation systems
    #[async_trait]
    pub trait Reputation: Send + Sync {
        /// Get the reputation score for an identity
        async fn get_reputation(&self, identity_id: &NodeId) -> ReputationResult<ReputationScore>;
        
        /// Submit evidence about an identity
        async fn submit_evidence(&self, evidence: Evidence) -> ReputationResult<()>;
        
        /// Get evidence for an identity
        async fn get_evidence(&self, identity_id: &NodeId) -> ReputationResult<Vec<Evidence>>;
        
        /// Get a specific piece of evidence by ID
        async fn get_evidence_by_id(&self, evidence_id: &str) -> ReputationResult<Option<Evidence>>;
        
        /// Verify evidence signature
        async fn verify_evidence(&self, evidence: &Evidence) -> ReputationResult<bool>;
    }
}

/// Default implementation of the Governance trait
pub struct DefaultGovernance<S> {
    /// The storage implementation
    storage: Arc<S>,
    /// The identity provider implementation
    identity_provider: Arc<dyn IdentityProvider>,
    /// Local node identity
    local_identity: NodeId,
    /// Governance configuration
    config: RwLock<GovernanceConfig>,
}

impl<S: Storage + 'static> DefaultGovernance<S> {
    /// Create a new DefaultGovernance instance
    pub fn new(
        storage: Arc<S>,
        identity_provider: Arc<dyn IdentityProvider>,
        local_identity: NodeId,
    ) -> Self {
        Self {
            storage,
            identity_provider,
            local_identity,
            config: RwLock::new(GovernanceConfig::default()),
        }
    }
    
    /// Get storage key for a proposal
    fn proposal_key(&self, id: &str) -> String {
        format!("governance:proposal:{}", id)
    }
    
    /// Get storage key for votes on a proposal
    fn votes_key(&self, proposal_id: &str) -> String {
        format!("governance:votes:{}", proposal_id)
    }
    
    /// Get storage key for the governance config
    fn config_key(&self) -> String {
        "governance:config".to_string()
    }
}

/// Implementation of the Governance trait for DefaultGovernance
#[async_trait]
impl<S: Storage + 'static> Governance for DefaultGovernance<S> {
    async fn create_proposal(&self, proposal: Proposal) -> GovernanceResult<Proposal> {
        // Implementation of create_proposal method
        Ok(proposal)
    }
    
    async fn cast_vote(&self, vote: Vote) -> GovernanceResult<Vote> {
        // Implementation of cast_vote method
        Ok(vote)
    }
    
    async fn get_proposal(&self, proposal_id: &str) -> GovernanceResult<Proposal> {
        // Implementation of get_proposal method
        Ok(Proposal {
            id: proposal_id.to_string(),
            title: "Stub Proposal".to_string(),
            description: "This is a stub proposal".to_string(),
            status: ProposalStatus::Open,
            created_at: icn_core::utils::timestamp_secs(),
            proposer: NodeId::from_string("stub-proposer"),
            proposal_type: ProposalType::Generic,
            voting_starts_at: icn_core::utils::timestamp_secs(),
            voting_ends_at: icn_core::utils::timestamp_secs() + 86400,
            processed_at: None,
            result: None,
            attributes: HashMap::new(),
            signature: Signature(Vec::new()),
        })
    }
    
    async fn get_votes(&self, proposal_id: &str) -> GovernanceResult<Vec<Vote>> {
        // Implementation of get_votes method
        Ok(Vec::new())
    }
    
    async fn execute_proposal(&self, proposal_id: &str) -> GovernanceResult<()> {
        // Implementation of execute_proposal method
        Ok(())
    }
    
    async fn cancel_proposal(&self, proposal_id: &str, reason: &str) -> GovernanceResult<()> {
        // Implementation of cancel_proposal method
        Ok(())
    }
}

// Implementation for IdentityError
impl From<icn_identity::IdentityError> for GovernanceError {
    fn from(err: icn_identity::IdentityError) -> Self {
        Self::IdentityError(err.to_string())
    }
} 