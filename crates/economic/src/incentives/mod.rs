use crate::error::Error;
use crate::assets::TokenManager;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};

/// Types of contributions that can be incentivized
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ContributionType {
    /// Node operation and maintenance
    NodeOperation,
    /// Consensus participation
    ConsensusParticipation,
    /// Content creation
    ContentCreation,
    /// Code development
    CodeDevelopment,
    /// Community moderation
    CommunityModeration,
    /// Resource sharing
    ResourceSharing,
    /// Governance participation
    GovernanceParticipation,
    /// Custom contribution type
    Custom(String),
}

/// A record of a contribution
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContributionRecord {
    /// ID of the contribution
    pub id: String,
    /// DID of the contributor
    pub contributor_did: String,
    /// Type of contribution
    pub contribution_type: ContributionType,
    /// Description of the contribution
    pub description: String,
    /// Timestamp of the contribution
    pub timestamp: DateTime<Utc>,
    /// Evidence of the contribution (e.g., links, hashes)
    pub evidence: Vec<String>,
    /// Status of the contribution
    pub status: ContributionStatus,
    /// Verification data
    pub verification: Option<ContributionVerification>,
    /// Reward details
    pub reward: Option<ContributionReward>,
    /// Federation ID (if applicable)
    pub federation_id: Option<String>,
    /// Metadata for the contribution
    pub metadata: HashMap<String, String>,
}

/// Status of a contribution
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ContributionStatus {
    /// Submitted but not yet verified
    Submitted,
    /// Under review
    UnderReview,
    /// Verified and approved
    Verified,
    /// Rejected
    Rejected,
    /// Rewarded
    Rewarded,
    /// Disputed
    Disputed,
}

/// Verification data for a contribution
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContributionVerification {
    /// DID of the verifier
    pub verifier_did: String,
    /// Timestamp of verification
    pub timestamp: DateTime<Utc>,
    /// Comments from the verifier
    pub comments: Option<String>,
    /// Score or rating (0.0 to 1.0)
    pub score: f64,
    /// Evidence provided by the verifier
    pub evidence: Vec<String>,
    /// Signatures from multiple verifiers if required
    pub signatures: Vec<String>,
}

/// Reward details for a contribution
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContributionReward {
    /// Token ID for the reward
    pub token_id: String,
    /// Amount of tokens awarded
    pub amount: f64,
    /// Transaction ID for the reward
    pub transaction_id: Option<String>,
    /// Timestamp of the reward
    pub timestamp: DateTime<Utc>,
    /// Formula used to calculate the reward
    pub formula: Option<String>,
    /// Multipliers applied to the base reward
    pub multipliers: HashMap<String, f64>,
}

/// Configuration for an incentive scheme
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IncentiveConfig {
    /// Name of the incentive scheme
    pub name: String,
    /// Description of the incentive scheme
    pub description: String,
    /// Contribution types incentivized by this scheme
    pub contribution_types: Vec<ContributionType>,
    /// Base reward rates by contribution type
    pub base_reward_rates: HashMap<ContributionType, f64>,
    /// Token ID used for rewards
    pub token_id: String,
    /// Whether reputation affects rewards
    pub reputation_based: bool,
    /// Verification requirements
    pub verification_requirements: VerificationRequirements,
    /// Cooldown period between contributions
    pub cooldown_period: Option<chrono::Duration>,
    /// Maximum rewards per time period
    pub reward_caps: HashMap<String, f64>,
    /// Whether federation membership affects rewards
    pub federation_aware: bool,
    /// Boost for early adopters
    pub early_adopter_boost: Option<f64>,
    /// Enabled status
    pub enabled: bool,
}

impl Default for IncentiveConfig {
    fn default() -> Self {
        Self {
            name: "Default Incentive Scheme".to_string(),
            description: "Default incentive scheme for basic contributions".to_string(),
            contribution_types: vec![ContributionType::NodeOperation, ContributionType::GovernanceParticipation],
            base_reward_rates: {
                let mut rates = HashMap::new();
                rates.insert(ContributionType::NodeOperation, 10.0);
                rates.insert(ContributionType::GovernanceParticipation, 5.0);
                rates
            },
            token_id: "ICN".to_string(),
            reputation_based: true,
            verification_requirements: VerificationRequirements::default(),
            cooldown_period: Some(chrono::Duration::hours(24)),
            reward_caps: {
                let mut caps = HashMap::new();
                caps.insert("daily".to_string(), 100.0);
                caps.insert("weekly".to_string(), 500.0);
                caps
            },
            federation_aware: false,
            early_adopter_boost: Some(1.2),
            enabled: true,
        }
    }
}

/// Verification requirements for contributions
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VerificationRequirements {
    /// Number of verifiers required
    pub min_verifiers: usize,
    /// Minimum reputation for verifiers
    pub min_verifier_reputation: Option<f64>,
    /// Minimum verification score
    pub min_verification_score: f64,
    /// Whether self-verification is allowed
    pub allow_self_verification: bool,
    /// Whether federation members get priority for verification
    pub federation_priority: bool,
}

impl Default for VerificationRequirements {
    fn default() -> Self {
        Self {
            min_verifiers: 1,
            min_verifier_reputation: Some(0.6),
            min_verification_score: 0.5,
            allow_self_verification: false,
            federation_priority: false,
        }
    }
}

/// A reward formula calculator
#[async_trait]
pub trait RewardCalculator: Send + Sync {
    /// Calculate a reward for a contribution
    async fn calculate_reward(
        &self,
        contribution: &ContributionRecord,
        contributor_reputation: f64,
        config: &IncentiveConfig,
    ) -> Result<ContributionReward, Error>;
}

/// Default implementation of a reward calculator
pub struct DefaultRewardCalculator;

#[async_trait]
impl RewardCalculator for DefaultRewardCalculator {
    async fn calculate_reward(
        &self,
        contribution: &ContributionRecord,
        contributor_reputation: f64,
        config: &IncentiveConfig,
    ) -> Result<ContributionReward, Error> {
        let base_rate = config.base_reward_rates
            .get(&contribution.contribution_type)
            .ok_or(Error::InvalidInput("No base rate for this contribution type".into()))?;
        
        let mut multipliers = HashMap::new();
        
        // Apply reputation multiplier if enabled
        if config.reputation_based {
            let reputation_multiplier = 0.5 + contributor_reputation * 0.5;
            multipliers.insert("reputation".to_string(), reputation_multiplier);
        }
        
        // Apply early adopter boost if applicable
        if let Some(boost) = config.early_adopter_boost {
            // Logic to determine if this contributor is an early adopter would go here
            let is_early_adopter = false; // Placeholder
            if is_early_adopter {
                multipliers.insert("early_adopter".to_string(), boost);
            }
        }
        
        // Calculate final amount
        let mut final_amount = *base_rate;
        for (_, multiplier) in &multipliers {
            final_amount *= multiplier;
        }
        
        Ok(ContributionReward {
            token_id: config.token_id.clone(),
            amount: final_amount,
            transaction_id: None,
            timestamp: chrono::Utc::now(),
            formula: Some(format!("base_rate({}) * multipliers", base_rate)),
            multipliers,
        })
    }
}

/// A contribution verification service
#[async_trait]
pub trait VerificationService: Send + Sync {
    /// Verify a contribution
    async fn verify_contribution(
        &self,
        contribution_id: &str,
        verifier_did: &str,
        score: f64,
        comments: Option<String>,
        evidence: Vec<String>,
    ) -> Result<ContributionVerification, Error>;
}

/// Manager for incentive mechanisms
pub struct IncentiveManager {
    /// Configuration for incentive schemes
    configs: RwLock<HashMap<String, IncentiveConfig>>,
    /// Contribution records
    contributions: RwLock<HashMap<String, ContributionRecord>>,
    /// Contributions by contributor
    contributor_contributions: RwLock<HashMap<String, HashSet<String>>>,
    /// Reward calculator
    reward_calculator: Arc<dyn RewardCalculator>,
    /// Verification service
    verification_service: Option<Arc<dyn VerificationService>>,
    /// Token manager for issuing rewards
    token_manager: Option<Arc<TokenManager>>,
}

impl IncentiveManager {
    /// Create a new incentive manager
    pub fn new(reward_calculator: Arc<dyn RewardCalculator>) -> Self {
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
    pub fn set_token_manager(&mut self, token_manager: Arc<TokenManager>) {
        self.token_manager = Some(token_manager);
    }
    
    /// Set the verification service
    pub fn set_verification_service(&mut self, verification_service: Arc<dyn VerificationService>) {
        self.verification_service = Some(verification_service);
    }
    
    /// Register an incentive scheme
    pub async fn register_incentive_scheme(
        &self,
        id: &str,
        config: IncentiveConfig,
    ) -> Result<(), Error> {
        self.configs.write().await.insert(id.to_string(), config);
        Ok(())
    }
    
    /// Get an incentive scheme configuration
    pub async fn get_incentive_config(&self, id: &str) -> Result<IncentiveConfig, Error> {
        let configs = self.configs.read().await;
        configs.get(id).cloned().ok_or(Error::NotFound)
    }
    
    /// Submit a contribution
    pub async fn submit_contribution(
        &self,
        contributor_did: &str,
        contribution_type: ContributionType,
        description: &str,
        evidence: Vec<String>,
        federation_id: Option<String>,
        metadata: HashMap<String, String>,
    ) -> Result<String, Error> {
        let id = format!("contrib_{}", uuid::Uuid::new_v4());
        
        let contribution = ContributionRecord {
            id: id.clone(),
            contributor_did: contributor_did.to_string(),
            contribution_type,
            description: description.to_string(),
            timestamp: chrono::Utc::now(),
            evidence,
            status: ContributionStatus::Submitted,
            verification: None,
            reward: None,
            federation_id,
            metadata,
        };
        
        // Store the contribution
        self.contributions.write().await.insert(id.clone(), contribution);
        
        // Update contributor index
        let mut contributor_contribs = self.contributor_contributions.write().await;
        let contribs = contributor_contribs
            .entry(contributor_did.to_string())
            .or_insert_with(HashSet::new);
        contribs.insert(id.clone());
        
        Ok(id)
    }
    
    /// Get a contribution by ID
    pub async fn get_contribution(&self, id: &str) -> Result<ContributionRecord, Error> {
        let contributions = self.contributions.read().await;
        contributions.get(id).cloned().ok_or(Error::NotFound)
    }
    
    /// Get all contributions for a contributor
    pub async fn get_contributor_contributions(&self, contributor_did: &str) -> Result<Vec<ContributionRecord>, Error> {
        let contrib_indices = self.contributor_contributions.read().await;
        let contributions = self.contributions.read().await;
        
        let result = match contrib_indices.get(contributor_did) {
            Some(ids) => {
                ids.iter()
                    .filter_map(|id| contributions.get(id).cloned())
                    .collect()
            }
            None => Vec::new(),
        };
        
        Ok(result)
    }
    
    /// Verify a contribution
    pub async fn verify_contribution(
        &self,
        contribution_id: &str,
        verifier_did: &str,
        score: f64,
        comments: Option<String>,
        evidence: Vec<String>,
    ) -> Result<(), Error> {
        // Get the verification service
        let verification_service = match &self.verification_service {
            Some(service) => service,
            None => return Err(Error::Internal("Verification service not set".into())),
        };
        
        // Verify the contribution
        let verification = verification_service.verify_contribution(
            contribution_id,
            verifier_did,
            score,
            comments,
            evidence,
        ).await?;
        
        // Update the contribution
        let mut contributions = self.contributions.write().await;
        let contribution = contributions.get_mut(contribution_id).ok_or(Error::NotFound)?;
        
        contribution.verification = Some(verification);
        contribution.status = ContributionStatus::Verified;
        
        Ok(())
    }
    
    /// Award a reward for a verified contribution
    pub async fn reward_contribution(
        &self,
        contribution_id: &str,
        scheme_id: &str,
        contributor_reputation: f64,
    ) -> Result<ContributionReward, Error> {
        // Get the token manager
        let token_manager = match &self.token_manager {
            Some(manager) => manager,
            None => return Err(Error::Internal("Token manager not set".into())),
        };
        
        // Get the contribution
        let mut contributions = self.contributions.write().await;
        let contribution = contributions.get_mut(contribution_id).ok_or(Error::NotFound)?;
        
        // Check if the contribution is verified
        if contribution.status != ContributionStatus::Verified {
            return Err(Error::InvalidState("Contribution not verified".into()));
        }
        
        // Get the incentive config
        let configs = self.configs.read().await;
        let config = configs.get(scheme_id).ok_or(Error::NotFound)?;
        
        // Calculate the reward
        let reward = self.reward_calculator.calculate_reward(
            contribution,
            contributor_reputation,
            config,
        ).await?;
        
        // Issue the reward
        // In a real implementation, this would use the token manager to issue tokens
        // token_manager.issue_tokens(&contribution.contributor_did, &reward.token_id, reward.amount).await?;
        
        // Update the contribution
        contribution.reward = Some(reward.clone());
        contribution.status = ContributionStatus::Rewarded;
        
        Ok(reward)
    }
    
    /// Get rewards summary for a contributor
    pub async fn get_contributor_rewards_summary(
        &self,
        contributor_did: &str,
    ) -> Result<HashMap<String, f64>, Error> {
        let contributions = self.get_contributor_contributions(contributor_did).await?;
        
        let mut token_totals = HashMap::new();
        
        for contribution in contributions {
            if let Some(reward) = contribution.reward {
                if contribution.status == ContributionStatus::Rewarded {
                    *token_totals.entry(reward.token_id).or_insert(0.0) += reward.amount;
                }
            }
        }
        
        Ok(token_totals)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // Tests would be implemented here
} 