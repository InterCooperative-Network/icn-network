//! Governance module for ICN
//!
//! This module provides decentralized governance capabilities for the
//! InterCooperative Network, including proposal creation, voting, and execution.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::fmt;

use async_trait::async_trait;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use serde::{Serialize, Deserialize};
use thiserror::Error;

use icn_core::{
    crypto::{Signature, Hash, identity::NodeId, sha256},
    storage::{Storage, StorageResult, StorageError, JsonStorage},
    utils::timestamp_secs,
};

use icn_identity::IdentityProvider;

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
    
    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    /// IO error
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    /// Not found error
    #[error("Not found")]
    NotFound,
    
    /// Invalid input
    #[error("Invalid input: {0}")]
    InvalidInput(String),
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

/// Type of proposal
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

// Manual implementation of Eq that ignores the floating point field
impl Eq for Vote {}

// Manual implementation of Hash that ignores the floating point field
impl std::hash::Hash for Vote {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.proposal_id.hash(state);
        self.voter.hash(state);
        self.approve.hash(state);
        self.comment.hash(state);
        // Skip self.weight since f64 doesn't implement Hash
        self.timestamp.hash(state);
        // Skip self.signature for now
    }
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
    
    /// Execute the proposal if it's approved
    async fn execute_proposal(&self, id: &str) -> GovernanceResult<()>;
}

pub mod execution;
pub mod voting;
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
    async fn get_config(&self) -> GovernanceResult<GovernanceConfig> {
        // Try to load from storage first
        match self.storage.get(&self.config_key()).await {
            Ok(data) => {
                if let Ok(config) = serde_json::from_slice::<GovernanceConfig>(&data) {
                    return Ok(config);
                }
            }
            Err(StorageError::KeyNotFound(_)) => {
                // Return the default config if not found
                return Ok(GovernanceConfig::default());
            }
            Err(e) => return Err(GovernanceError::StorageError(e)),
        }
        
        // If we get here, there was an error deserializing
        // Return the in-memory config instead
        let config = self.config.read().await.clone();
        Ok(config)
    }
    
    async fn set_config(&self, config: GovernanceConfig) -> GovernanceResult<()> {
        // Update the in-memory config
        *self.config.write().await = config.clone();
        
        // Save to storage
        let data = serde_json::to_vec(&config)
            .map_err(|e| GovernanceError::SerializationError(e.to_string()))?;
            
        self.storage.put(&self.config_key(), &data).await
            .map_err(GovernanceError::StorageError)?;
            
        Ok(())
    }
    
    async fn create_proposal(
        &self,
        title: String,
        description: String,
        proposal_type: ProposalType,
        voting_period: Option<u64>,
        attributes: HashMap<String, String>,
    ) -> GovernanceResult<Proposal> {
        let config = self.get_config().await?;
        let now = timestamp_secs();
        
        // Calculate voting period
        let voting_period = voting_period.unwrap_or(config.default_voting_period);
        let voting_starts_at = now;
        let voting_ends_at = now + voting_period;
        
        // Create proposal
        let mut proposal = Proposal::new(
            title,
            description,
            proposal_type,
            self.local_identity.clone(),
            voting_starts_at,
            voting_ends_at,
            attributes,
        );
        
        // Sign the proposal
        let bytes = proposal.bytes_to_sign();
        let signature_bytes = self.identity_provider.sign(&bytes).await
            .map_err(|e| GovernanceError::IdentityError(format!("Failed to sign proposal: {:?}", e)))?;
        proposal.signature = Signature(signature_bytes);
        
        // Save the proposal
        let data = serde_json::to_vec(&proposal)
            .map_err(|e| GovernanceError::SerializationError(e.to_string()))?;
            
        self.storage.put(&self.proposal_key(&proposal.id), &data).await
            .map_err(GovernanceError::StorageError)?;
        
        Ok(proposal)
    }
    
    async fn get_proposal(&self, id: &str) -> GovernanceResult<Option<Proposal>> {
        match self.storage.get(&self.proposal_key(id)).await {
            Ok(data) => {
                let proposal = serde_json::from_slice::<Proposal>(&data)
                    .map_err(|e| GovernanceError::SerializationError(e.to_string()))?;
                Ok(Some(proposal))
            }
            Err(StorageError::KeyNotFound(_)) => Ok(None),
            Err(e) => Err(GovernanceError::StorageError(e)),
        }
    }
    
    async fn list_proposals(&self) -> GovernanceResult<Vec<Proposal>> {
        // We'll use a prefix to get all proposals
        let prefix = "governance:proposal:";
        let result = self.storage.list(&prefix).await
            .map_err(GovernanceError::StorageError)?;
            
        let mut proposals = Vec::new();
        for key in result {
            match self.storage.get(&key).await {
                Ok(data) => {
                    if let Ok(proposal) = serde_json::from_slice::<Proposal>(&data) {
                        proposals.push(proposal);
                    }
                }
                Err(_) => continue,
            }
        }
        
        Ok(proposals)
    }
    
    async fn vote(
        &self,
        proposal_id: &str,
        approve: bool,
        comment: Option<String>,
    ) -> GovernanceResult<Vote> {
        // Get the proposal
        let proposal = match self.get_proposal(proposal_id).await? {
            Some(p) => p,
            None => return Err(GovernanceError::ProposalNotFound(proposal_id.to_string())),
        };
        
        // Check if voting is open
        if !proposal.is_open_for_voting() {
            return Err(GovernanceError::InvalidVote(
                "Voting is not open for this proposal".to_string()
            ));
        }
        
        // Create the vote
        let mut vote = Vote::new(
            proposal_id.to_string(),
            self.local_identity.clone(),
            approve,
            comment,
            None, // No weight for now
        );
        
        // Sign the vote
        let bytes = vote.bytes_to_sign();
        let signature_bytes = self.identity_provider.sign(&bytes).await
            .map_err(|e| GovernanceError::IdentityError(format!("Failed to sign vote: {:?}", e)))?;
        vote.signature = Signature(signature_bytes);
        
        // Save the vote
        let votes_key = self.votes_key(proposal_id);
        
        // Get existing votes
        let mut votes: Vec<Vote> = match self.storage.get(&votes_key).await {
            Ok(data) => serde_json::from_slice(&data)
                .map_err(|e| GovernanceError::SerializationError(e.to_string()))?,
            Err(StorageError::KeyNotFound(_)) => Vec::new(),
            Err(e) => return Err(GovernanceError::StorageError(e)),
        };
        
        // Check if the user already voted
        let voter_id = self.local_identity.to_string();
        if votes.iter().any(|v| v.voter.to_string() == voter_id) {
            return Err(GovernanceError::InvalidVote("Already voted".to_string()));
        }
        
        // Add the new vote
        votes.push(vote.clone());
        
        // Save all votes
        let data = serde_json::to_vec(&votes)
            .map_err(|e| GovernanceError::SerializationError(e.to_string()))?;
            
        self.storage.put(&votes_key, &data).await
            .map_err(GovernanceError::StorageError)?;
        
        Ok(vote)
    }
    
    async fn get_votes(&self, proposal_id: &str) -> GovernanceResult<Vec<Vote>> {
        let votes_key = self.votes_key(proposal_id);
        
        match self.storage.get(&votes_key).await {
            Ok(data) => {
                let votes = serde_json::from_slice::<Vec<Vote>>(&data)
                    .map_err(|e| GovernanceError::SerializationError(e.to_string()))?;
                Ok(votes)
            }
            Err(StorageError::KeyNotFound(_)) => Ok(Vec::new()),
            Err(e) => Err(GovernanceError::StorageError(e)),
        }
    }
    
    async fn process_proposal(&self, proposal_id: &str) -> GovernanceResult<ProposalStatus> {
        // Get the proposal
        let mut proposal = match self.get_proposal(proposal_id).await? {
            Some(p) => p,
            None => return Err(GovernanceError::ProposalNotFound(proposal_id.to_string())),
        };
        
        // Check if voting is closed
        if !proposal.is_voting_closed() {
            return Err(GovernanceError::InvalidProposal("Voting is not closed".to_string()));
        }
        
        // If already processed, return current status
        if proposal.status == ProposalStatus::Approved || 
           proposal.status == ProposalStatus::Rejected {
            return Ok(proposal.status);
        }
        
        // Get the votes
        let votes = self.get_votes(proposal_id).await?;
        
        // Get the config for quorum and approval percentage
        let config = self.get_config().await?;
        
        // Count the votes
        let mut yes_votes = 0;
        let mut no_votes = 0;
        
        for vote in &votes {
            if vote.approve {
                yes_votes += 1;
            } else {
                no_votes += 1;
            }
        }
        
        // Calculate quorum
        let total_votes = yes_votes + no_votes;
        let total_possible_votes = 100; // placeholder, should be total members with voting rights
        
        let quorum_reached = (total_votes as f64 / total_possible_votes as f64) >= config.quorum_percentage;
        
        // Calculate approval
        let approval_reached = quorum_reached && 
            (yes_votes as f64 / total_votes as f64) >= config.approval_percentage;
        
        // Update proposal status
        if !quorum_reached {
            proposal.status = ProposalStatus::Rejected;
            proposal.result = Some("Quorum not reached".to_string());
        } else if approval_reached {
            proposal.status = ProposalStatus::Approved;
            proposal.result = Some(format!(
                "Approved with {}/{} yes votes ({}%)", 
                yes_votes, 
                total_votes,
                (yes_votes as f64 / total_votes as f64) * 100.0
            ));
        } else {
            proposal.status = ProposalStatus::Rejected;
            proposal.result = Some(format!(
                "Rejected with {}/{} yes votes ({}%)", 
                yes_votes, 
                total_votes,
                (yes_votes as f64 / total_votes as f64) * 100.0
            ));
        }
        
        proposal.processed_at = Some(timestamp_secs());
        
        // Save the updated proposal
        let data = serde_json::to_vec(&proposal)
            .map_err(|e| GovernanceError::SerializationError(e.to_string()))?;
            
        self.storage.put(&self.proposal_key(&proposal.id), &data).await
            .map_err(GovernanceError::StorageError)?;
        
        Ok(proposal.status)
    }
    
    async fn cancel_proposal(&self, proposal_id: &str) -> GovernanceResult<()> {
        // Get the proposal
        let mut proposal = match self.get_proposal(proposal_id).await? {
            Some(p) => p,
            None => return Err(GovernanceError::ProposalNotFound(proposal_id.to_string())),
        };
        
        // Check if the user is the proposer
        let is_proposer = proposal.proposer.to_string() == self.local_identity.to_string();
        
        // For simplicity, only the proposer can cancel for now
        if !is_proposer {
            return Err(GovernanceError::PermissionDenied(
                "Only the proposer can cancel a proposal".to_string()
            ));
        }
        
        // Check if the proposal can be cancelled
        if proposal.status == ProposalStatus::Executed || 
           proposal.status == ProposalStatus::Failed {
            return Err(GovernanceError::InvalidProposal(
                "Proposal cannot be cancelled in its current state".to_string()
            ));
        }
        
        // Update the status
        proposal.status = ProposalStatus::Cancelled;
        proposal.processed_at = Some(timestamp_secs());
        
        // Save the updated proposal
        let data = serde_json::to_vec(&proposal)
            .map_err(|e| GovernanceError::SerializationError(e.to_string()))?;
            
        self.storage.put(&self.proposal_key(&proposal.id), &data).await
            .map_err(GovernanceError::StorageError)?;
        
        Ok(())
    }
    
    async fn execute_proposal(&self, id: &str) -> GovernanceResult<()> {
        // Implementation of execute_proposal method
        Ok(())
    }
}

// Implementation for IdentityError
impl From<icn_identity::IdentityError> for GovernanceError {
    fn from(err: icn_identity::IdentityError) -> Self {
        Self::IdentityError(err.to_string())
    }
} 