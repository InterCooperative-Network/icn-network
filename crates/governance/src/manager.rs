//! Governance manager implementation
//!
//! This module provides the implementation of the Governance trait,
//! managing proposals, votes, and governance processes.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use serde::{Serialize, Deserialize};
use async_trait::async_trait;

use icn_core::{
    storage::Storage,
    crypto::{identity::NodeId, Signature, verify_signature},
    utils::timestamp_secs,
};

use icn_identity::service::{
    IdentityService as IdentityProvider, 
    Identity
};

// Local type definition
type IdentityResult<T> = Result<T, crate::GovernanceError>;

use crate::{
    Governance, GovernanceConfig, GovernanceResult, GovernanceError,
    Proposal, ProposalStatus, ProposalType, Vote,
    voting::{VotingScheme, SimpleVoting, WeightedVoting},
    execution::ProposalExecutor,
};

/// Path constants for storage
const CONFIG_PATH: &str = "governance/config";
const PROPOSALS_PATH: &str = "governance/proposals";
const VOTES_PATH: &str = "governance/votes";

/// The main implementation of the Governance trait
pub struct GovernanceManager {
    /// Identity provider for authentication and signatures
    identity_provider: Arc<dyn IdentityProvider>,
    /// Reputation system for determining voting weights
    reputation: Arc<dyn Reputation>,
    /// Storage for governance data
    storage: Arc<dyn Storage>,
    /// Current configuration
    config: Arc<RwLock<GovernanceConfig>>,
    /// Proposals cache (by ID)
    proposals: Arc<RwLock<HashMap<String, Proposal>>>,
    /// Votes cache (proposal ID -> set of votes)
    votes: Arc<RwLock<HashMap<String, HashSet<Vote>>>>,
    /// Voting scheme
    voting_scheme: Arc<RwLock<Box<dyn VotingScheme>>>,
    /// Proposal executor
    executor: Arc<dyn ProposalExecutor>,
}

impl GovernanceManager {
    /// Create a new governance manager
    pub async fn new(
        identity_provider: Arc<dyn IdentityProvider>,
        reputation: Arc<dyn Reputation>,
        storage: Arc<dyn Storage>,
        executor: Arc<dyn ProposalExecutor>,
    ) -> GovernanceResult<Self> {
        // Load configuration
        let config = Self::load_config(&storage).await?;
        
        // Create voting scheme based on config
        let voting_scheme: Box<dyn VotingScheme> = if config.use_weighted_voting {
            Box::new(WeightedVoting::new(
                config.quorum_percentage, 
                config.approval_percentage,
            ))
        } else {
            Box::new(SimpleVoting::new(
                config.quorum_percentage, 
                config.approval_percentage,
            ))
        };
        
        let manager = Self {
            identity_provider,
            reputation,
            storage,
            config: Arc::new(RwLock::new(config)),
            proposals: Arc::new(RwLock::new(HashMap::new())),
            votes: Arc::new(RwLock::new(HashMap::new())),
            voting_scheme: Arc::new(RwLock::new(voting_scheme)),
            executor,
        };
        
        // Load existing proposals and votes
        manager.load_proposals().await?;
        manager.load_votes().await?;
        
        Ok(manager)
    }
    
    /// Load configuration from storage
    async fn load_config(storage: &dyn Storage) -> GovernanceResult<GovernanceConfig> {
        match storage.get_json::<GovernanceConfig>(CONFIG_PATH).await {
            Ok(config) => Ok(config),
            Err(_) => {
                // If no config exists, use default and save it
                let config = GovernanceConfig::default();
                if let Err(e) = storage.put_json(CONFIG_PATH, &config).await {
                    warn!("Failed to save default governance configuration: {}", e);
                }
                Ok(config)
            }
        }
    }
    
    /// Load proposals from storage
    async fn load_proposals(&self) -> GovernanceResult<()> {
        let proposal_keys = self.storage.list(PROPOSALS_PATH).await?;
        let mut proposals = self.proposals.write().await;
        
        for key in proposal_keys {
            match self.storage.get_json::<Proposal>(&key).await {
                Ok(proposal) => {
                    proposals.insert(proposal.id.clone(), proposal);
                },
                Err(e) => {
                    error!("Failed to load proposal {}: {}", key, e);
                }
            }
        }
        
        info!("Loaded {} proposals", proposals.len());
        Ok(())
    }
    
    /// Load votes from storage
    async fn load_votes(&self) -> GovernanceResult<()> {
        let vote_keys = self.storage.list(VOTES_PATH).await?;
        let mut votes = self.votes.write().await;
        
        for key in vote_keys {
            match self.storage.get_json::<Vec<Vote>>(&key).await {
                Ok(proposal_votes) => {
                    let proposal_id = key.strip_prefix(VOTES_PATH).unwrap_or(&key);
                    let proposal_id = proposal_id.trim_start_matches('/');
                    
                    let vote_set = votes.entry(proposal_id.to_string())
                        .or_insert_with(HashSet::new);
                    
                    for vote in proposal_votes {
                        vote_set.insert(vote);
                    }
                },
                Err(e) => {
                    error!("Failed to load votes for {}: {}", key, e);
                }
            }
        }
        
        info!("Loaded votes for {} proposals", votes.len());
        Ok(())
    }
    
    /// Save a proposal to storage
    async fn save_proposal(&self, proposal: &Proposal) -> GovernanceResult<()> {
        let path = format!("{}/{}", PROPOSALS_PATH, proposal.id);
        self.storage.put_json(&path, proposal).await?;
        
        // Update cache
        let mut proposals = self.proposals.write().await;
        proposals.insert(proposal.id.clone(), proposal.clone());
        
        Ok(())
    }
    
    /// Save votes for a proposal to storage
    async fn save_votes(&self, proposal_id: &str) -> GovernanceResult<()> {
        let votes = self.votes.read().await;
        if let Some(vote_set) = votes.get(proposal_id) {
            let path = format!("{}/{}", VOTES_PATH, proposal_id);
            let vote_vec: Vec<Vote> = vote_set.iter().cloned().collect();
            self.storage.put_json(&path, &vote_vec).await?;
        }
        Ok(())
    }
    
    /// Verify if a user has sufficient reputation to create a proposal
    async fn verify_proposal_permission(&self, identity_id: &NodeId) -> GovernanceResult<bool> {
        let config = self.config.read().await;
        
        // Get the user's reputation
        let reputation_score = match self.reputation.get_reputation(identity_id).await {
            Ok(score) => score,
            Err(e) => {
                return Err(GovernanceError::ReputationError(e));
            }
        };
        
        // Check if they meet the minimum reputation requirement
        if reputation_score.score < config.min_proposal_reputation {
            return Ok(false);
        }
        
        Ok(true)
    }
    
    /// Verify if a user has sufficient reputation to vote
    async fn verify_voting_permission(&self, identity_id: &NodeId) -> GovernanceResult<bool> {
        let config = self.config.read().await;
        
        // Get the user's reputation
        let reputation_score = match self.reputation.get_reputation(identity_id).await {
            Ok(score) => score,
            Err(e) => {
                return Err(GovernanceError::ReputationError(e));
            }
        };
        
        // Check if they meet the minimum reputation requirement
        if reputation_score.score < config.min_voting_reputation {
            return Ok(false);
        }
        
        Ok(true)
    }
    
    /// Generate a positive reputation evidence for governance participation
    async fn add_governance_participation_evidence(
        &self, 
        identity_id: &NodeId,
        activity_type: &str,
        description: &str,
        weight: f64,
    ) -> GovernanceResult<()> {
        // Create evidence data
        let mut data = HashMap::new();
        data.insert("activity_type".to_string(), activity_type.to_string());
        
        // Use the current node's identity as the submitter
        let submitter = self.identity_provider.get_identity().await?;
        
        // Create evidence
        let evidence = Evidence::new(
            submitter.id.clone(),
            identity_id.clone(),
            EvidenceType::GovernanceParticipation,
            description.to_string(),
            weight,
            data,
            Vec::new(),
        );
        
        // Submit the evidence
        match self.reputation.submit_evidence(evidence).await {
            Ok(_) => Ok(()),
            Err(e) => Err(GovernanceError::ReputationError(e)),
        }
    }
    
    /// Update the voting scheme based on the current configuration
    async fn update_voting_scheme(&self) -> GovernanceResult<()> {
        let config = self.config.read().await;
        let mut voting_scheme = self.voting_scheme.write().await;
        
        *voting_scheme = if config.use_weighted_voting {
            Box::new(WeightedVoting::new(
                config.quorum_percentage, 
                config.approval_percentage,
            ))
        } else {
            Box::new(SimpleVoting::new(
                config.quorum_percentage, 
                config.approval_percentage,
            ))
        };
        
        Ok(())
    }
    
    /// Check and update proposals that need status changes
    pub async fn process_pending_proposals(&self) -> GovernanceResult<()> {
        let now = timestamp_secs();
        let mut proposals_to_process = Vec::new();
        
        // Find proposals that need processing
        {
            let proposals = self.proposals.read().await;
            for (id, proposal) in proposals.iter() {
                if proposal.status == ProposalStatus::Open && proposal.voting_ends_at < now {
                    proposals_to_process.push(id.clone());
                }
            }
        }
        
        // Process each proposal
        for proposal_id in proposals_to_process {
            match self.process_proposal(&proposal_id).await {
                Ok(status) => {
                    info!("Automatically processed proposal {}, new status: {:?}", proposal_id, status);
                },
                Err(e) => {
                    error!("Failed to automatically process proposal {}: {}", proposal_id, e);
                }
            }
        }
        
        Ok(())
    }
}

#[async_trait]
impl Governance for GovernanceManager {
    async fn get_config(&self) -> GovernanceResult<GovernanceConfig> {
        let config = self.config.read().await;
        Ok(config.clone())
    }
    
    async fn set_config(&self, config: GovernanceConfig) -> GovernanceResult<()> {
        // Save to storage first
        self.storage.put_json(CONFIG_PATH, &config).await?;
        
        // Update local cache
        {
            let mut local_config = self.config.write().await;
            *local_config = config;
        }
        
        // Update voting scheme
        self.update_voting_scheme().await?;
        
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
        // Get the current user's identity
        let identity = self.identity_provider.get_identity().await?;
        
        // Verify permission
        if !self.verify_proposal_permission(&identity.id).await? {
            return Err(GovernanceError::PermissionDenied(
                format!("Insufficient reputation to create a proposal")
            ));
        }
        
        // Get the voting period duration
        let config = self.config.read().await;
        let voting_period = voting_period.unwrap_or(config.default_voting_period);
        
        let now = timestamp_secs();
        
        // Create the proposal
        let mut proposal = Proposal::new(
            title,
            description,
            proposal_type,
            identity.id.clone(),
            now,
            now + voting_period,
            attributes,
        );
        
        // Set status to Open
        proposal.status = ProposalStatus::Open;
        
        // Sign the proposal
        let bytes_to_sign = proposal.bytes_to_sign();
        let signature = self.identity_provider.sign(&bytes_to_sign).await?;
        proposal.signature = signature;
        
        // Save the proposal
        self.save_proposal(&proposal).await?;
        
        // Add governance participation evidence
        self.add_governance_participation_evidence(
            &identity.id,
            "proposal_creation",
            &format!("Created governance proposal: {}", proposal.title),
            config.proposal_creation_reputation,
        ).await?;
        
        Ok(proposal)
    }
    
    async fn get_proposal(&self, id: &str) -> GovernanceResult<Option<Proposal>> {
        let proposals = self.proposals.read().await;
        Ok(proposals.get(id).cloned())
    }
    
    async fn list_proposals(&self) -> GovernanceResult<Vec<Proposal>> {
        let proposals = self.proposals.read().await;
        let mut result: Vec<Proposal> = proposals.values().cloned().collect();
        
        // Sort by creation time, newest first
        result.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        
        Ok(result)
    }
    
    async fn vote(
        &self,
        proposal_id: &str,
        approve: bool,
        comment: Option<String>,
    ) -> GovernanceResult<Vote> {
        // Get the current user's identity
        let identity = self.identity_provider.get_identity().await?;
        
        // Verify permission
        if !self.verify_voting_permission(&identity.id).await? {
            return Err(GovernanceError::PermissionDenied(
                format!("Insufficient reputation to vote on proposals")
            ));
        }
        
        // Get the proposal
        let proposal = match self.get_proposal(proposal_id).await? {
            Some(p) => p,
            None => return Err(GovernanceError::ProposalNotFound(proposal_id.to_string())),
        };
        
        // Check if the proposal is open for voting
        if !proposal.is_open_for_voting() {
            return Err(GovernanceError::InvalidVote(
                format!("Proposal is not open for voting")
            ));
        }
        
        // Get the user's reputation for weighted voting
        let config = self.config.read().await;
        let weight = if config.use_weighted_voting {
            match self.reputation.get_reputation(&identity.id).await {
                Ok(score) => Some(score.score),
                Err(e) => return Err(GovernanceError::ReputationError(e)),
            }
        } else {
            None
        };
        
        // Create the vote
        let mut vote = Vote::new(
            proposal_id.to_string(),
            identity.id.clone(),
            approve,
            comment,
            weight,
        );
        
        // Sign the vote
        let bytes_to_sign = vote.bytes_to_sign();
        let signature = self.identity_provider.sign(&bytes_to_sign).await?;
        vote.signature = signature;
        
        // Add the vote to cache
        {
            let mut votes = self.votes.write().await;
            let vote_set = votes.entry(proposal_id.to_string())
                .or_insert_with(HashSet::new);
            vote_set.insert(vote.clone());
        }
        
        // Save the votes
        self.save_votes(proposal_id).await?;
        
        // Add governance participation evidence
        self.add_governance_participation_evidence(
            &identity.id,
            "vote_cast",
            &format!("Voted on proposal: {}", proposal.title),
            config.voting_reputation,
        ).await?;
        
        Ok(vote)
    }
    
    async fn get_votes(&self, proposal_id: &str) -> GovernanceResult<Vec<Vote>> {
        let votes = self.votes.read().await;
        
        match votes.get(proposal_id) {
            Some(vote_set) => {
                let mut result: Vec<Vote> = vote_set.iter().cloned().collect();
                
                // Sort by timestamp, newest first
                result.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
                
                Ok(result)
            },
            None => Ok(Vec::new()),
        }
    }
    
    async fn process_proposal(&self, proposal_id: &str) -> GovernanceResult<ProposalStatus> {
        // Get the proposal
        let mut proposal = match self.get_proposal(proposal_id).await? {
            Some(p) => p,
            None => return Err(GovernanceError::ProposalNotFound(proposal_id.to_string())),
        };
        
        // Check if the proposal is ready to be processed
        if proposal.status != ProposalStatus::Open && proposal.status != ProposalStatus::Closed {
            return Err(GovernanceError::InvalidProposal(
                format!("Proposal is not in a state that can be processed")
            ));
        }
        
        // If the voting period is not over, close it first
        if !proposal.is_voting_closed() && proposal.status == ProposalStatus::Open {
            proposal.status = ProposalStatus::Closed;
            self.save_proposal(&proposal).await?;
            return Ok(ProposalStatus::Closed);
        }
        
        // Get all votes for this proposal
        let votes = self.get_votes(proposal_id).await?;
        
        // Process the votes using the voting scheme
        let voting_scheme = self.voting_scheme.read().await;
        let result = voting_scheme.tally_votes(&votes)?;
        
        // Update the proposal based on the voting result
        if result.approved {
            // Update proposal status
            proposal.status = ProposalStatus::Approved;
            proposal.processed_at = Some(timestamp_secs());
            proposal.result = Some(format!(
                "Approved with {:.1}% yes votes ({} yes, {} no, {:.1}% participation)",
                result.approval_percentage * 100.0,
                result.yes_votes,
                result.no_votes,
                result.participation_percentage * 100.0
            ));
            
            // Execute the proposal
            if let Err(e) = self.executor.execute_proposal(&proposal).await {
                error!("Failed to execute approved proposal {}: {}", proposal_id, e);
                proposal.status = ProposalStatus::Failed;
                proposal.result = Some(format!("Execution failed: {}", e));
            } else {
                proposal.status = ProposalStatus::Executed;
            }
        } else {
            // Mark as rejected
            proposal.status = ProposalStatus::Rejected;
            proposal.processed_at = Some(timestamp_secs());
            
            if !result.has_quorum {
                proposal.result = Some(format!(
                    "Rejected due to insufficient participation ({:.1}% < {:.1}% required)",
                    result.participation_percentage * 100.0,
                    result.quorum_percentage * 100.0
                ));
            } else {
                proposal.result = Some(format!(
                    "Rejected with {:.1}% yes votes ({} yes, {} no, {:.1}% participation)",
                    result.approval_percentage * 100.0,
                    result.yes_votes,
                    result.no_votes,
                    result.participation_percentage * 100.0
                ));
            }
        }
        
        // Save the updated proposal
        self.save_proposal(&proposal).await?;
        
        Ok(proposal.status)
    }
    
    async fn cancel_proposal(&self, proposal_id: &str) -> GovernanceResult<()> {
        // Get the current user's identity
        let identity = self.identity_provider.get_identity().await?;
        
        // Get the proposal
        let mut proposal = match self.get_proposal(proposal_id).await? {
            Some(p) => p,
            None => return Err(GovernanceError::ProposalNotFound(proposal_id.to_string())),
        };
        
        // Check if the proposal can be cancelled (only if it's Draft or Open)
        if proposal.status != ProposalStatus::Draft && proposal.status != ProposalStatus::Open {
            return Err(GovernanceError::InvalidProposal(
                format!("Proposal cannot be cancelled in its current state")
            ));
        }
        
        // Check if the user is the proposer
        if proposal.proposer != identity.id {
            // TODO: Check if the user is an admin
            return Err(GovernanceError::PermissionDenied(
                format!("Only the proposer can cancel this proposal")
            ));
        }
        
        // Cancel the proposal
        proposal.status = ProposalStatus::Cancelled;
        proposal.processed_at = Some(timestamp_secs());
        proposal.result = Some(format!("Cancelled by proposer"));
        
        // Save the updated proposal
        self.save_proposal(&proposal).await?;
        
        Ok(())
    }
} 