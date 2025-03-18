use std::collections::HashMap;
use std::time::{Duration, Instant};
use libp2p::PeerId;
use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use crate::reputation::ReputationContext;

/// Configuration for the enhanced reputation system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedReputationConfig {
    /// Minimum reputation score
    pub min_score: i32,
    /// Maximum reputation score
    pub max_score: i32,
    /// Default starting score
    pub default_score: i32,
    /// Score decay rate per hour
    pub decay_rate: f32,
    /// Threshold for considering a peer as trusted
    pub trusted_threshold: i32,
    /// Threshold for banning a peer
    pub ban_threshold: i32,
}

impl Default for EnhancedReputationConfig {
    fn default() -> Self {
        Self {
            min_score: -100,
            max_score: 100,
            default_score: 0,
            decay_rate: 0.5,
            trusted_threshold: 50,
            ban_threshold: -50,
        }
    }
}

/// Interaction value with a peer
#[derive(Debug, Clone, Copy)]
pub enum InteractionValue {
    /// Positive interaction
    Positive(i32),
    /// Negative interaction
    Negative(i32),
    /// Neutral interaction
    Neutral,
}

/// Metrics for reputation tracking
#[derive(Debug, Clone, Default)]
pub struct ReputationMetrics {
    /// Total positive interactions
    pub positive_interactions: u32,
    /// Total negative interactions
    pub negative_interactions: u32,
    /// Average score change per interaction
    pub avg_score_change: f32,
    /// Last interaction time
    pub last_interaction: Option<Instant>,
}

/// Handler for reputation events
#[async_trait]
pub trait ReputationEventHandler: Send + Sync {
    /// Handle a reputation change event
    async fn handle_reputation_change(&self, peer_id: &PeerId, context: ReputationContext, value: InteractionValue);
}

/// Provider of contribution metrics
#[async_trait]
pub trait ContributionMetricsProvider: Send + Sync {
    /// Get contribution metrics for a peer
    async fn get_contribution_metrics(&self, peer_id: &PeerId) -> HashMap<String, f64>;
}

/// Enhanced reputation manager
#[derive(Debug)]
pub struct EnhancedReputationManager {
    /// Reputation scores by peer ID and context
    scores: HashMap<PeerId, HashMap<ReputationContext, i32>>,
    /// Reputation metrics by peer ID
    metrics: HashMap<PeerId, ReputationMetrics>,
    /// Reputation configuration
    config: EnhancedReputationConfig,
}

impl EnhancedReputationManager {
    /// Create a new enhanced reputation manager
    pub fn new(config: EnhancedReputationConfig) -> Self {
        Self {
            scores: HashMap::new(),
            metrics: HashMap::new(),
            config,
        }
    }
    
    /// Update a peer's reputation
    pub fn update_reputation(&mut self, peer_id: &PeerId, context: ReputationContext, value: InteractionValue) {
        let context_scores = self.scores.entry(*peer_id).or_insert_with(HashMap::new);
        let score = context_scores.entry(context).or_insert(self.config.default_score);
        
        match value {
            InteractionValue::Positive(val) => {
                *score = (*score + val).min(self.config.max_score);
                let metrics = self.metrics.entry(*peer_id).or_insert_with(ReputationMetrics::default);
                metrics.positive_interactions += 1;
            },
            InteractionValue::Negative(val) => {
                *score = (*score - val).max(self.config.min_score);
                let metrics = self.metrics.entry(*peer_id).or_insert_with(ReputationMetrics::default);
                metrics.negative_interactions += 1;
            },
            InteractionValue::Neutral => {},
        }
        
        // Update last interaction time
        if let Some(metrics) = self.metrics.get_mut(peer_id) {
            metrics.last_interaction = Some(Instant::now());
        }
    }
    
    /// Get a peer's reputation score for a specific context
    pub fn get_reputation(&self, peer_id: &PeerId, context: &ReputationContext) -> i32 {
        self.scores
            .get(peer_id)
            .and_then(|contexts| contexts.get(context))
            .copied()
            .unwrap_or(self.config.default_score)
    }
    
    /// Check if a peer is trusted
    pub fn is_trusted(&self, peer_id: &PeerId) -> bool {
        self.get_reputation(peer_id, &ReputationContext::Networking) >= self.config.trusted_threshold
    }
    
    /// Check if a peer is banned
    pub fn is_banned(&self, peer_id: &PeerId) -> bool {
        self.get_reputation(peer_id, &ReputationContext::Networking) <= self.config.ban_threshold
    }
}

/// Alias for ReputationChange for backward compatibility
pub type ReputationChange = InteractionValue; 