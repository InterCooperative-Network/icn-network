//! Incentive mechanisms for the ICN economic system.
//!
//! This module implements incentive mechanisms to encourage participation and contributions
//! in the network, including storage provision, data sharing, and governance participation.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use chrono::{DateTime, Utc};
use async_trait::async_trait;
use uuid::Uuid;
use serde::{Deserialize, Serialize};

use crate::EconomicError;
use crate::Result;

/// Types of actions that can be incentivized in the network
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IncentiveActionType {
    /// Storage provision on the network
    StorageProvision,
    
    /// Sharing of data with other members
    DataSharing,
    
    /// Participation in governance activities
    GovernanceParticipation,
    
    /// Node operation and hosting
    NodeOperation,
    
    /// Content creation and curation
    ContentCreation,
    
    /// Network bootstrapping and growth
    NetworkGrowth,
    
    /// Reputation vouching for new members
    ReputationVouching,
    
    /// Other customizable actions
    Custom,
}

impl std::fmt::Display for IncentiveActionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StorageProvision => write!(f, "storage_provision"),
            Self::DataSharing => write!(f, "data_sharing"),
            Self::GovernanceParticipation => write!(f, "governance_participation"),
            Self::NodeOperation => write!(f, "node_operation"),
            Self::ContentCreation => write!(f, "content_creation"),
            Self::NetworkGrowth => write!(f, "network_growth"),
            Self::ReputationVouching => write!(f, "reputation_vouching"),
            Self::Custom => write!(f, "custom"),
        }
    }
}

/// Configuration for an incentive model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncentiveModelConfig {
    /// Name of the incentive model
    pub name: String,
    
    /// Description of the incentive model
    pub description: String,
    
    /// Base reward rates for each action type
    pub base_rates: HashMap<IncentiveActionType, f64>,
    
    /// Time-based multipliers (e.g., boost for early adopters)
    pub time_multipliers: HashMap<String, f64>,
    
    /// Reputation-based multipliers
    pub reputation_multipliers: HashMap<String, f64>,
    
    /// Whether this incentive model is active
    pub active: bool,
}

/// A record of a contribution eligible for incentives
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContributionRecord {
    /// Unique ID for this contribution
    pub id: String,
    
    /// ID of the contributor
    pub contributor_id: String,
    
    /// Type of contribution
    pub action_type: IncentiveActionType,
    
    /// When the contribution was made
    pub timestamp: DateTime<Utc>,
    
    /// Quantitative measure of the contribution (e.g., bytes stored, votes cast)
    pub quantity: f64,
    
    /// Optional qualitative score (0-100)
    pub quality_score: Option<u8>,
    
    /// Whether this contribution has been verified
    pub verified: bool,
    
    /// Whether a reward has been issued for this contribution
    pub rewarded: bool,
    
    /// Verification details if verified
    pub verification: Option<ContributionVerification>,
    
    /// Additional metadata about the contribution
    pub metadata: HashMap<String, String>,
}

impl ContributionRecord {
    /// Create a new contribution record
    pub fn new(
        contributor_id: String,
        action_type: IncentiveActionType,
        quantity: f64,
        _account_id: &str,
        _metadata: Option<HashMap<String, String>>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            contributor_id,
            action_type,
            timestamp: Utc::now(),
            quantity,
            quality_score: None,
            verified: false,
            rewarded: false,
            verification: None,
            metadata: HashMap::new(),
        }
    }
}

/// Verification of a contribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContributionVerification {
    /// ID of the verifier
    pub verifier_id: String,
    
    /// Verification score (0-100)
    pub score: u8,
    
    /// Comments from the verifier
    pub comments: Option<String>,
    
    /// When the verification was performed
    pub timestamp: DateTime<Utc>,
    
    /// Any evidence provided
    pub evidence: Option<HashMap<String, String>>,
}

/// Reward for a contribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContributionReward {
    /// ID of the contribution
    pub contribution_id: String,
    
    /// ID of the contributor
    pub contributor_id: String,
    
    /// Base amount
    pub base_amount: f64,
    
    /// Reputation multiplier applied
    pub reputation_multiplier: f64,
    
    /// Time-based multiplier applied
    pub time_multiplier: f64,
    
    /// Total reward amount
    pub total_amount: f64,
    
    /// When the reward was calculated
    pub timestamp: DateTime<Utc>,
    
    /// Token ID if using multiple token types
    pub token_id: Option<String>,
    
    /// Transaction ID if recorded on-chain
    pub transaction_id: Option<String>,
}

/// Configuration for an incentive scheme
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncentiveConfig {
    /// Unique ID for this incentive scheme
    pub id: String,
    
    /// Name of the incentive scheme
    pub name: String,
    
    /// Description of the scheme
    pub description: String,
    
    /// Base rates for different contribution types
    pub base_rates: HashMap<IncentiveActionType, f64>,
    
    /// Minimum threshold for rewards
    pub min_threshold: f64,
    
    /// Whether verification is required
    pub requires_verification: bool,
    
    /// Reputation tiers and their multipliers
    pub reputation_tiers: HashMap<String, f64>,
    
    /// Whether this scheme is active
    pub active: bool,
    
    /// When this scheme was created
    pub created_at: DateTime<Utc>,
    
    /// When this scheme was last updated
    pub updated_at: DateTime<Utc>,
}

/// A calculator for incentive rewards
#[async_trait]
pub trait RewardCalculator: Send + Sync {
    /// Calculate a reward for a contribution
    async fn calculate_reward(
        &self,
        contribution: &ContributionRecord,
        contributor_reputation: f64,
        incentive_config: &IncentiveConfig,
    ) -> Result<ContributionReward>;
}

/// Implementation of a default reward calculator
pub struct DefaultRewardCalculator;

#[async_trait]
impl RewardCalculator for DefaultRewardCalculator {
    async fn calculate_reward(
        &self,
        contribution: &ContributionRecord,
        contributor_reputation: f64,
        incentive_config: &IncentiveConfig,
    ) -> Result<ContributionReward> {
        let base_rate = incentive_config.base_rates.get(&contribution.action_type)
            .ok_or(EconomicError::InvalidInput("No base rate for this contribution type".into()))?;
            
        // Calculate base amount
        let base_amount = base_rate * contribution.quantity;
        
        // Calculate reputation multiplier
        let reputation_multiplier = if contributor_reputation < 25.0 {
            0.8
        } else if contributor_reputation < 50.0 {
            1.0
        } else if contributor_reputation < 75.0 {
            1.2
        } else {
            1.5
        };
        
        // Calculate time-based multiplier (just using 1.0 for now)
        let time_multiplier = 1.0;
        
        // Calculate total reward
        let total_amount = base_amount * reputation_multiplier * time_multiplier;
        
        Ok(ContributionReward {
            contribution_id: contribution.id.clone(),
            contributor_id: contribution.contributor_id.clone(),
            base_amount,
            reputation_multiplier,
            time_multiplier,
            total_amount,
            timestamp: Utc::now(),
            token_id: None,
            transaction_id: None,
        })
    }
}

/// Service for verifying contributions
#[async_trait]
pub trait VerificationService: Send + Sync {
    /// Verify a contribution
    async fn verify_contribution(
        &self,
        contribution_id: &str,
        verifier_id: &str,
        score: u8,
        comments: Option<String>,
        evidence: Option<HashMap<String, String>>,
    ) -> Result<ContributionVerification>;
}

/// Manager for handling incentives
pub struct IncentiveManager {
    /// Incentive configurations
    configs: RwLock<HashMap<String, IncentiveConfig>>,
    
    /// Recorded contributions
    contributions: RwLock<HashMap<String, ContributionRecord>>,
    
    /// Map of contributor IDs to their contribution IDs
    contributor_contributions: RwLock<HashMap<String, HashSet<String>>>,
    
    /// Reward calculator
    reward_calculator: Arc<DefaultRewardCalculator>,
    
    /// Verification service
    verification_service: Option<Arc<dyn VerificationService + Send + Sync>>,
    
    /// Token manager for issuing rewards
    token_manager: Option<Arc<dyn Send + Sync>>,
}

impl IncentiveManager {
    /// Create a new incentive manager
    pub fn new(reward_calculator: Arc<DefaultRewardCalculator>) -> Self {
        Self {
            configs: RwLock::new(HashMap::new()),
            contributions: RwLock::new(HashMap::new()),
            contributor_contributions: RwLock::new(HashMap::new()),
            reward_calculator,
            verification_service: None,
            token_manager: None,
        }
    }
    
    /// Set the token manager
    pub fn set_token_manager(&mut self, token_manager: Arc<dyn Send + Sync>) {
        self.token_manager = Some(token_manager);
    }
    
    /// Set the verification service
    pub fn set_verification_service(&mut self, verification_service: Arc<dyn VerificationService + Send + Sync>) {
        self.verification_service = Some(verification_service);
    }
    
    /// Add a new incentive configuration
    pub async fn add_incentive_config(
        &self,
        name: String,
        description: String,
        base_rates: HashMap<IncentiveActionType, f64>,
        requires_verification: bool,
    ) -> Result<()> {
        let config = IncentiveConfig {
            id: Uuid::new_v4().to_string(),
            name,
            description,
            base_rates,
            min_threshold: 1.0,
            requires_verification,
            reputation_tiers: HashMap::new(),
            active: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        
        self.configs.write().unwrap().insert(config.id.clone(), config);
        Ok(())
    }
    
    /// Get an incentive config by ID
    pub async fn get_incentive_config(&self, id: &str) -> Result<IncentiveConfig> {
        let configs = self.configs.read().unwrap();
        configs.get(id).cloned().ok_or(EconomicError::NotFound(format!("Incentive config not found with id: {}", id)))
    }
    
    /// Register a new contribution
    pub async fn register_contribution(
        &self,
        contributor_id: String,
        action_type: IncentiveActionType,
        quantity: f64,
        account_id: &str,
        metadata: Option<HashMap<String, String>>,
    ) -> Result<String> {
        // Create a new contribution record
        let contribution = ContributionRecord::new(
            contributor_id.clone(),
            action_type,
            quantity,
            account_id,
            metadata,
        );
        
        // Store the contribution
        let contribution_id = contribution.id.clone();
        
        // Add to the contributions map
        self.contributions.write().unwrap().insert(contribution_id.clone(), contribution);
        
        // Add to the contributor's set of contributions
        self.contributor_contributions
            .write()
            .unwrap()
            .entry(contributor_id)
            .or_insert_with(HashSet::new)
            .insert(contribution_id.clone());
        
        Ok(contribution_id)
    }
    
    /// Get a contribution by ID
    pub async fn get_contribution(&self, id: &str) -> Result<ContributionRecord> {
        let contributions = self.contributions.read().unwrap();
        contributions.get(id).cloned().ok_or(EconomicError::NotFound(format!("Contribution not found with id: {}", id)))
    }
    
    /// Get all contributions for a contributor
    pub async fn get_contributor_contributions(&self, contributor_id: &str) -> Result<Vec<ContributionRecord>> {
        let contributor_map = self.contributor_contributions.read().unwrap();
        let contributions = self.contributions.read().unwrap();
        
        match contributor_map.get(contributor_id) {
            Some(contribution_ids) => {
                let mut result = Vec::new();
                for id in contribution_ids {
                    if let Some(contribution) = contributions.get(id) {
                        result.push(contribution.clone());
                    }
                }
                Ok(result)
            }
            None => Ok(Vec::new()),
        }
    }
    
    /// Verify a contribution
    pub async fn verify_contribution(
        &self,
        contribution_id: &str,
        verifier_did: &str,
        score: u8,
        comments: Option<String>,
        evidence: Option<HashMap<String, String>>,
    ) -> Result<()> {
        // Get the verification service
        let verification_service = match &self.verification_service {
            Some(service) => service,
            None => return Err(EconomicError::Internal("Verification service not set".into())),
        };
        
        // Verify the contribution
        let verification = verification_service.verify_contribution(
            contribution_id,
            verifier_did,
            score,
            comments,
            evidence,
        ).await?;
        
        // Update the contribution record
        let mut contributions = self.contributions.write().unwrap();
        let contribution = contributions.get_mut(contribution_id).ok_or(EconomicError::NotFound(format!("Contribution not found with id: {}", contribution_id)))?;
        
        contribution.verified = true;
        contribution.verification = Some(verification);
        
        Ok(())
    }
    
    /// Calculate reward for a contribution
    pub async fn calculate_reward(
        &self,
        contribution_id: &str,
        scheme_id: &str,
        contributor_reputation: f64,
    ) -> Result<ContributionReward> {
        // Get the token manager
        let _token_manager = match &self.token_manager {
            Some(manager) => manager,
            None => return Err(EconomicError::Internal("Token manager not set".into())),
        };
        
        // Get the contribution
        let mut contributions = self.contributions.write().unwrap();
        let contribution = contributions.get_mut(contribution_id).ok_or(EconomicError::NotFound(format!("Contribution not found with id: {}", contribution_id)))?;
        
        // Check if verified if required
        if !contribution.verified {
            return Err(EconomicError::InvalidState("Contribution not verified".into()));
        }
        
        // Get the incentive config
        let configs = self.configs.read().unwrap();
        let config = configs.get(scheme_id).ok_or(EconomicError::NotFound(format!("Incentive config not found with id: {}", scheme_id)))?;
        
        // Calculate the reward
        let reward = self.reward_calculator.calculate_reward(
            contribution,
            contributor_reputation,
            config,
        ).await?;
        
        // Mark as rewarded
        contribution.rewarded = true;
        
        Ok(reward)
    }
    
    /// Get contribution stats for a period
    pub async fn get_contribution_stats(
        &self,
        contributor_id: Option<&str>,
        action_type: Option<IncentiveActionType>,
        since: Option<DateTime<Utc>>,
    ) -> Result<HashMap<String, f64>> {
        let contributions = self.contributions.read().unwrap();
        
        let mut stats = HashMap::new();
        let mut total_count = 0.0;
        let mut verified_count = 0.0;
        let mut rewarded_count = 0.0;
        let mut total_quantity = 0.0;
        
        for contribution in contributions.values() {
            // Apply filters
            if let Some(cid) = contributor_id {
                if contribution.contributor_id != cid {
                    continue;
                }
            }
            
            if let Some(at) = action_type {
                if contribution.action_type != at {
                    continue;
                }
            }
            
            if let Some(since_time) = since {
                if contribution.timestamp < since_time {
                    continue;
                }
            }
            
            // Update stats
            total_count += 1.0;
            total_quantity += contribution.quantity;
            
            if contribution.verified {
                verified_count += 1.0;
            }
            
            if contribution.rewarded {
                rewarded_count += 1.0;
            }
        }
        
        stats.insert("total_count".to_string(), total_count);
        stats.insert("verified_count".to_string(), verified_count);
        stats.insert("rewarded_count".to_string(), rewarded_count);
        stats.insert("total_quantity".to_string(), total_quantity);
        
        Ok(stats)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_incentive_action_type_display() {
        assert_eq!(IncentiveActionType::StorageProvision.to_string(), "storage_provision");
        assert_eq!(IncentiveActionType::DataSharing.to_string(), "data_sharing");
        assert_eq!(IncentiveActionType::GovernanceParticipation.to_string(), "governance_participation");
    }
    
    // More tests will be added as needed
} 