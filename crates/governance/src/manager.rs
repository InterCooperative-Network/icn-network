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
use serde::de::DeserializeOwned;
use async_trait::async_trait;

use icn_core::{
    storage::{Storage, StorageResult, StorageError, JsonStorage},
    config::ConfigProvider,
    crypto::{identity::NodeId, Signature, verify_signature},
    utils::timestamp_secs,
};

use icn_identity::IdentityProvider;

// Local type definitions
type IdentityResult<T> = Result<T, crate::GovernanceError>;

// Import reputation types from our local module
use crate::reputation::{Reputation, Evidence, EvidenceType, ReputationScore};

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
    
    /// Helper method to get JSON data
    async fn get_json<T: DeserializeOwned + Send>(&self, key: &str) -> StorageResult<T> {
        let data = self.storage.get(key).await?;
        serde_json::from_slice(&data)
            .map_err(|e| StorageError::DeserializationError(e.to_string()))
    }
    
    /// Helper method to put JSON data
    async fn put_json<T: Serialize + Send + Sync>(&self, key: &str, value: &T) -> StorageResult<()> {
        let json_data = serde_json::to_vec_pretty(value)
            .map_err(|e| StorageError::SerializationError(e.to_string()))?;
        self.storage.put(key, &json_data).await
    }
    
    /// Load configuration from storage
    async fn load_config(storage: &Arc<dyn Storage>) -> GovernanceResult<GovernanceConfig> {
        let storage_ref = storage.as_ref();
        // Try to load existing config
        match storage_ref.get(CONFIG_PATH).await {
            Ok(data) => {
                // Successfully loaded raw data, deserialize
                match serde_json::from_slice::<GovernanceConfig>(&data) {
                    Ok(config) => Ok(config),
                    Err(e) => Err(GovernanceError::SerializationError(e.to_string())),
                }
            }
            Err(StorageError::KeyNotFound(_)) => {
                // Config not found, create default
                let config = GovernanceConfig::default();
                
                // Persist the new config
                let json_data = serde_json::to_vec_pretty(&config)
                    .map_err(|e| GovernanceError::SerializationError(e.to_string()))?;
                
                if let Err(e) = storage_ref.put(CONFIG_PATH, &json_data).await {
                    error!("Failed to save default governance config: {}", e);
                }
                
                Ok(config)
            }
            Err(e) => {
                // Other error
                Err(GovernanceError::StorageError(e))
            }
        }
    }
    
    /// Load existing proposals from storage
    async fn load_proposals(&self) -> GovernanceResult<()> {
        let prefix = format!("{}/", PROPOSALS_PATH);
        let keys = self.storage.list(&prefix).await?;
        
        let mut proposals = self.proposals.write().await;
        for key in keys {
            match self.get_json::<Proposal>(&key).await {
                Ok(proposal) => {
                    proposals.insert(proposal.id.clone(), proposal);
                },
                Err(e) => {
                    error!("Failed to load proposal {}: {}", key, e);
                    // Continue to next proposal
                }
            }
        }
        
        info!("Loaded {} proposals", proposals.len());
        Ok(())
    }
    
    /// Load existing votes from storage
    async fn load_votes(&self) -> GovernanceResult<()> {
        let prefix = format!("{}/", VOTES_PATH);
        let keys = self.storage.list(&prefix).await?;
        
        let mut votes = self.votes.write().await;
        for key in keys {
            match self.get_json::<Vec<Vote>>(&key).await {
                Ok(vote_vec) => {
                    let vote_set: HashSet<Vote> = vote_vec.into_iter().collect();
                    
                    if let Some(proposal_id) = key.strip_prefix(&prefix) {
                        votes.insert(proposal_id.to_string(), vote_set);
                    }
                },
                Err(e) => {
                    error!("Failed to load votes {}: {}", key, e);
                    // Continue to next vote set
                }
            }
        }
        
        info!("Loaded votes for {} proposals", votes.len());
        Ok(())
    }
    
    /// Save proposal to storage
    async fn save_proposal(&self, proposal: &Proposal) -> GovernanceResult<()> {
        let path = format!("{}/{}", PROPOSALS_PATH, proposal.id);
        self.put_json(&path, proposal).await?;
        Ok(())
    }
    
    /// Save votes for a proposal to storage
    async fn save_votes(&self, proposal_id: &str, votes: &HashSet<Vote>) -> GovernanceResult<()> {
        let path = format!("{}/{}", VOTES_PATH, proposal_id);
        let vote_vec: Vec<Vote> = votes.iter().cloned().collect();
        
        if !vote_vec.is_empty() {
            self.put_json(&path, &vote_vec).await?;
        } else {
            // If no votes, delete the entry
            if let Err(e) = self.storage.delete(&path).await {
                // Ignore KeyNotFound errors
                if !matches!(e, StorageError::KeyNotFound(_)) {
                    return Err(GovernanceError::StorageError(e));
                }
            }
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
                return Err(GovernanceError::ReputationError(e.to_string()));
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
                return Err(GovernanceError::ReputationError(e.to_string()));
            }
        };
        
        // Check if they meet the minimum reputation requirement
        if reputation_score.score < config.min_voting_reputation {
            return Ok(false);
        }
        
        Ok(true)
    }
    
    /// Add evidence of governance participation to the reputation system
    async fn add_governance_participation_evidence(
        &self, 
        identity_id: &NodeId,
        activity_type: &str,
        description: &str,
        weight: f64,
    ) {
        // Convert parameters to the evidence type format
        let evidence_type = match activity_type {
            "vote_cast" => EvidenceType::Voting,
            "proposal_creation" => EvidenceType::ProposalCreation,
            "proposal_execution" => EvidenceType::ProposalExecution,
            _ => return, // Unknown activity type
        };
        
        // Create evidence and submit to reputation system
        let evidence = Evidence::new(
            identity_id.clone(),
            identity_id.clone(), // Subject is the same as submitter for self-reported evidence
            evidence_type,
            description.to_string(),
            weight,
        );
        
        if let Err(e) = self.reputation.submit_evidence(evidence).await {
            error!("Failed to submit governance participation evidence: {}", e);
        } else {
            info!("Added governance participation evidence for {}", identity_id);
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
    /// Get the governance configuration
    async fn get_config(&self) -> GovernanceResult<GovernanceConfig> {
        let config = self.config.read().await;
        Ok((*config).clone())
    }
    
    /// Set the governance configuration
    async fn set_config(&self, config: GovernanceConfig) -> GovernanceResult<()> {
        {
            // Update the in-memory config
            let mut cfg = self.config.write().await;
            *cfg = config.clone();
            
            // Update the voting scheme if needed
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
        }
        
        // Persist to storage
        self.put_json(CONFIG_PATH, &config).await?;
        
        Ok(())
    }
    
    /// Create a new proposal
    async fn create_proposal(
        &self,
        title: String,
        description: String,
        proposal_type: ProposalType,
        voting_period: Option<u64>,
        attributes: HashMap<String, String>,
    ) -> GovernanceResult<Proposal> {
        // Validation and implementation...
        todo!()
    }
    
    /// Get a proposal by ID
    async fn get_proposal(&self, id: &str) -> GovernanceResult<Option<Proposal>> {
        // Lookup and return proposal
        let proposals = self.proposals.read().await;
        Ok(proposals.get(id).cloned())
    }
    
    /// List all proposals
    async fn list_proposals(&self) -> GovernanceResult<Vec<Proposal>> {
        // Return list of proposals
        let proposals = self.proposals.read().await;
        let mut result: Vec<Proposal> = proposals.values().cloned().collect();
        
        // Sort by creation time, newest first
        result.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        
        Ok(result)
    }
    
    /// Cast a vote on a proposal
    async fn vote(
        &self,
        proposal_id: &str,
        approve: bool,
        comment: Option<String>,
    ) -> GovernanceResult<Vote> {
        // Validate proposal exists and is open for voting
        let proposal = match self.get_proposal(proposal_id).await? {
            Some(p) => p,
            None => return Err(GovernanceError::ProposalNotFound(proposal_id.to_string())),
        };
        
        // Check proposal status
        if proposal.status != ProposalStatus::Open {
            return Err(GovernanceError::InvalidProposal(
                format!("Proposal is not open for voting: {:?}", proposal.status)
            ));
        }
        
        // Verify voter identity
        let identity = self.identity_provider.get_identity().await
            .map_err(|e| GovernanceError::IdentityError(e.to_string()))?;
        
        // Convert String to NodeId
        let voter_node_id = NodeId::from_string(identity.id.clone());
        
        if !self.verify_voting_permission(&voter_node_id).await? {
            return Err(GovernanceError::PermissionDenied(
                "Voter does not have permission to vote".into()
            ));
        }
        
        // Get reputation score for weighted voting
        let mut weight = None;
        if self.config.read().await.use_weighted_voting {
            match self.reputation.get_reputation(&voter_node_id).await {
                Ok(score) => weight = Some(score.score),
                Err(e) => return Err(GovernanceError::ReputationError(e.to_string())),
            }
        }
        
        // Create the vote
        let mut vote = Vote::new(
            proposal_id.to_string(),
            voter_node_id.clone(),
            approve,
            comment,
            weight,
        );
        
        // Sign the vote
        let bytes_to_sign = serde_json::to_vec(&vote)
            .map_err(|e| GovernanceError::SerializationError(e.to_string()))?;
        
        let signature_bytes = self.identity_provider.sign(&bytes_to_sign).await
            .map_err(|e| GovernanceError::IdentityError(e.to_string()))?;
        
        // Convert the Vec<u8> to a Signature
        vote.signature = Signature(signature_bytes);
        
        // Save the vote
        {
            let mut votes = self.votes.write().await;
            let vote_set = votes.entry(proposal_id.to_string())
                .or_insert_with(HashSet::new);
            
            vote_set.insert(vote.clone());
            
            // Save the votes to storage
            self.save_votes(proposal_id, vote_set).await?;
        }
        
        // Add governance participation evidence
        self.add_governance_participation_evidence(
            &voter_node_id,
            "vote_cast",
            &format!("Voted on proposal: {}", proposal.title),
            1.0,  // Default reputation impact
        ).await;
        
        // Return the vote
        Ok(vote)
    }
    
    /// Get votes for a proposal
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
    
    /// Process a proposal after voting is complete
    async fn process_proposal(&self, proposal_id: &str) -> GovernanceResult<ProposalStatus> {
        // Implementation goes here
        todo!()
    }
    
    /// Execute a proposal
    async fn execute_proposal(&self, id: &str) -> GovernanceResult<()> {
        // Execute the proposal if it's approved
        todo!()
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
        if proposal.proposer != NodeId::from_string(&identity.id) {
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