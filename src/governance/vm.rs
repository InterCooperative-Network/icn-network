use super::primitives::*;
use super::storage::GovernanceStore;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::error::Error;

pub struct GovernanceVM {
    store: Arc<dyn GovernanceStore>,
    action_executor: Arc<dyn ActionExecutor>,
    voting_strategy: Arc<dyn VotingStrategy>,
    // Cache layers for frequently accessed data
    identities_cache: Arc<RwLock<HashMap<String, Identity>>>,
    proposals_cache: Arc<RwLock<HashMap<String, Proposal>>>,
    votes_cache: Arc<RwLock<HashMap<String, Vec<Vote>>>>,
}

impl GovernanceVM {
    pub fn new(
        store: Arc<dyn GovernanceStore>,
        action_executor: Arc<dyn ActionExecutor>,
        voting_strategy: Arc<dyn VotingStrategy>,
    ) -> Self {
        Self {
            store,
            action_executor,
            voting_strategy,
            identities_cache: Arc::new(RwLock::new(HashMap::new())),
            proposals_cache: Arc::new(RwLock::new(HashMap::new())),
            votes_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register_identity(&self, identity: Identity) -> Result<(), Box<dyn Error>> {
        // Store in persistent storage
        self.store.put(&format!("identities/{}", identity.id), &identity).await?;
        
        // Update cache
        let mut cache = self.identities_cache.write().await;
        cache.insert(identity.id.clone(), identity);
        Ok(())
    }

    pub async fn submit_proposal(&self, proposal: Proposal) -> Result<(), Box<dyn Error>> {
        // Validate proposer exists
        let identity = self.get_identity(&proposal.proposer).await?
            .ok_or("Proposer not found")?;

        // Store proposal
        self.store.put(&format!("proposals/{}", proposal.id), &proposal).await?;
        
        // Initialize empty vote collection
        self.store.put(&format!("votes/{}", proposal.id), &Vec::<Vote>::new()).await?;
        
        // Update caches
        let mut proposals = self.proposals_cache.write().await;
        proposals.insert(proposal.id.clone(), proposal);
        
        let mut votes = self.votes_cache.write().await;
        votes.insert(proposal.id.clone(), Vec::new());
        
        Ok(())
    }

    pub async fn cast_vote(&self, vote: Vote) -> Result<(), Box<dyn Error>> {
        // Validate voter exists
        let voter = self.get_identity(&vote.voter).await?
            .ok_or("Voter not found")?;

        // Validate proposal exists and is active
        let proposal = self.get_proposal(&vote.proposal_id).await?
            .ok_or("Proposal not found")?;
        
        if proposal.state != ProposalState::Active {
            return Err("Proposal is not active".into());
        }

        // Get current votes
        let mut proposal_votes = self.get_votes(&vote.proposal_id).await?
            .unwrap_or_default();
        
        // Add new vote
        proposal_votes.push(vote);
        
        // Store updated votes
        self.store.put(&format!("votes/{}", proposal.id), &proposal_votes).await?;
        
        // Update cache
        let mut votes = self.votes_cache.write().await;
        votes.insert(proposal.id.clone(), proposal_votes.clone());

        // Check if we should tally votes
        let total_voters = self.count_total_voters().await?;
        let vote_count = proposal_votes.len() as f64;
        
        if vote_count / total_voters as f64 >= proposal.voting_method.quorum {
            // We have quorum, tally the votes
            let result = self.voting_strategy.tally_votes(
                &proposal_votes,
                &proposal.voting_method
            ).await?;

            // Update proposal state
            let mut updated_proposal = proposal.clone();
            updated_proposal.state = result.clone();
            
            // Store updated proposal
            self.store.put(&format!("proposals/{}", proposal.id), &updated_proposal).await?;
            
            // Update cache
            let mut proposals = self.proposals_cache.write().await;
            proposals.insert(proposal.id.clone(), updated_proposal.clone());

            // If proposal passed, execute its actions
            if result == ProposalState::Passed {
                for action in &updated_proposal.execution {
                    self.action_executor.execute(action).await?;
                }
            } else if result == ProposalState::Rejected {
                for action in &updated_proposal.rejection {
                    self.action_executor.execute(action).await?;
                }
            }
        }

        Ok(())
    }

    pub async fn get_proposal(&self, id: &str) -> Result<Option<Proposal>, Box<dyn Error>> {
        // Check cache first
        if let Some(proposal) = self.proposals_cache.read().await.get(id) {
            return Ok(Some(proposal.clone()));
        }
        
        // If not in cache, check storage
        if let Some(proposal) = self.store.get(&format!("proposals/{}", id)).await? {
            // Update cache
            let mut cache = self.proposals_cache.write().await;
            cache.insert(id.to_string(), proposal.clone());
            Ok(Some(proposal))
        } else {
            Ok(None)
        }
    }

    pub async fn get_identity(&self, id: &str) -> Result<Option<Identity>, Box<dyn Error>> {
        // Check cache first
        if let Some(identity) = self.identities_cache.read().await.get(id) {
            return Ok(Some(identity.clone()));
        }
        
        // If not in cache, check storage
        if let Some(identity) = self.store.get(&format!("identities/{}", id)).await? {
            // Update cache
            let mut cache = self.identities_cache.write().await;
            cache.insert(id.to_string(), identity.clone());
            Ok(Some(identity))
        } else {
            Ok(None)
        }
    }

    pub async fn get_votes(&self, proposal_id: &str) -> Result<Option<Vec<Vote>>, Box<dyn Error>> {
        // Check cache first
        if let Some(votes) = self.votes_cache.read().await.get(proposal_id) {
            return Ok(Some(votes.clone()));
        }
        
        // If not in cache, check storage
        if let Some(votes) = self.store.get(&format!("votes/{}", proposal_id)).await? {
            // Update cache
            let mut cache = self.votes_cache.write().await;
            cache.insert(proposal_id.to_string(), votes.clone());
            Ok(Some(votes))
        } else {
            Ok(None)
        }
    }

    async fn count_total_voters(&self) -> Result<usize, Box<dyn Error>> {
        let keys = self.store.list("identities/").await?;
        Ok(keys.len())
    }
} 