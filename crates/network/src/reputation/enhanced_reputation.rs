use std::collections::HashMap;
use std::time::{Duration, Instant};
use std::sync::Arc;
use libp2p::PeerId;
use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use crate::reputation::ReputationContext;
use tokio::sync::RwLock;

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
    /// Connection established
    ConnectionEstablished,
    /// Connection lost
    ConnectionLost,
    /// Message success
    MessageSuccess,
    /// Message failure
    MessageFailure,
    /// Invalid message
    InvalidMessage,
    /// Verified message
    VerifiedMessage,
    /// Discovery help
    DiscoveryHelp,
    /// Relay success
    RelaySuccess,
    /// Relay failure
    RelayFailure,
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
    scores: Arc<RwLock<HashMap<PeerId, HashMap<ReputationContext, i32>>>>,
    /// Reputation metrics by peer ID
    metrics: Arc<RwLock<HashMap<PeerId, ReputationMetrics>>>,
    /// Reputation configuration
    config: EnhancedReputationConfig,
    /// Banned peers
    banned_peers: Arc<RwLock<HashMap<PeerId, Instant>>>,
}

impl EnhancedReputationManager {
    /// Create a new enhanced reputation manager
    pub fn new(config: EnhancedReputationConfig) -> Self {
        Self {
            scores: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(HashMap::new())),
            config,
            banned_peers: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Update a peer's reputation
    async fn update_reputation(&self, peer_id: &PeerId, context: ReputationContext, value: InteractionValue) -> crate::NetworkResult<()> {
        let mut scores = self.scores.write().await;
        let context_scores = scores.entry(*peer_id).or_insert_with(HashMap::new);
        let score = context_scores.entry(context).or_insert(self.config.default_score);
        
        match value {
            InteractionValue::Positive(val) => {
                *score = (*score + val).min(self.config.max_score);
                let mut metrics = self.metrics.write().await;
                let metrics_entry = metrics.entry(*peer_id).or_insert_with(ReputationMetrics::default);
                metrics_entry.positive_interactions += 1;
            },
            InteractionValue::Negative(val) => {
                *score = (*score - val).max(self.config.min_score);
                let mut metrics = self.metrics.write().await;
                let metrics_entry = metrics.entry(*peer_id).or_insert_with(ReputationMetrics::default);
                metrics_entry.negative_interactions += 1;
            },
            InteractionValue::Neutral => {},
            InteractionValue::ConnectionEstablished => {
                *score = (*score + 1).min(self.config.max_score);
                let mut metrics = self.metrics.write().await;
                let metrics_entry = metrics.entry(*peer_id).or_insert_with(ReputationMetrics::default);
                metrics_entry.positive_interactions += 1;
            },
            InteractionValue::ConnectionLost => {
                // No penalty for normal connection loss
            },
            InteractionValue::MessageSuccess => {
                *score = (*score + 1).min(self.config.max_score);
                let mut metrics = self.metrics.write().await;
                let metrics_entry = metrics.entry(*peer_id).or_insert_with(ReputationMetrics::default);
                metrics_entry.positive_interactions += 1;
            },
            InteractionValue::MessageFailure => {
                *score = (*score - 1).max(self.config.min_score);
                let mut metrics = self.metrics.write().await;
                let metrics_entry = metrics.entry(*peer_id).or_insert_with(ReputationMetrics::default);
                metrics_entry.negative_interactions += 1;
            },
            InteractionValue::InvalidMessage => {
                *score = (*score - 2).max(self.config.min_score);
                let mut metrics = self.metrics.write().await;
                let metrics_entry = metrics.entry(*peer_id).or_insert_with(ReputationMetrics::default);
                metrics_entry.negative_interactions += 1;
            },
            InteractionValue::VerifiedMessage => {
                *score = (*score + 1).min(self.config.max_score);
                let mut metrics = self.metrics.write().await;
                let metrics_entry = metrics.entry(*peer_id).or_insert_with(ReputationMetrics::default);
                metrics_entry.positive_interactions += 1;
            },
            InteractionValue::DiscoveryHelp => {
                *score = (*score + 1).min(self.config.max_score);
                let mut metrics = self.metrics.write().await;
                let metrics_entry = metrics.entry(*peer_id).or_insert_with(ReputationMetrics::default);
                metrics_entry.positive_interactions += 1;
            },
            InteractionValue::RelaySuccess => {
                *score = (*score + 2).min(self.config.max_score);
                let mut metrics = self.metrics.write().await;
                let metrics_entry = metrics.entry(*peer_id).or_insert_with(ReputationMetrics::default);
                metrics_entry.positive_interactions += 1;
            },
            InteractionValue::RelayFailure => {
                *score = (*score - 2).max(self.config.min_score);
                let mut metrics = self.metrics.write().await;
                let metrics_entry = metrics.entry(*peer_id).or_insert_with(ReputationMetrics::default);
                metrics_entry.negative_interactions += 1;
            },
        }
        
        // Update last interaction time
        {
            let mut metrics = self.metrics.write().await;
            if let Some(metrics_entry) = metrics.get_mut(peer_id) {
                metrics_entry.last_interaction = Some(Instant::now());
            }
        }
        
        // Check if the peer should be banned
        if let Some(score) = scores.get(peer_id).and_then(|ctx_scores| ctx_scores.get(&ReputationContext::Networking)) {
            if *score <= self.config.ban_threshold {
                let mut banned = self.banned_peers.write().await;
                banned.insert(*peer_id, Instant::now());
            }
        }
        
        Ok(())
    }
    
    /// Record a reputation change for a peer
    pub async fn record_change(&self, peer_id: PeerId, change: InteractionValue) -> crate::NetworkResult<()> {
        self.update_reputation(&peer_id, ReputationContext::Networking, change).await
    }
    
    /// Ban a peer
    pub async fn ban_peer(&self, peer_id: PeerId) -> crate::NetworkResult<()> {
        {
            let mut banned = self.banned_peers.write().await;
            banned.insert(peer_id, Instant::now());
        }
        
        // Set reputation to minimum
        let mut scores = self.scores.write().await;
        let context_scores = scores.entry(peer_id).or_insert_with(HashMap::new);
        let score = context_scores.entry(ReputationContext::Networking).or_insert(self.config.default_score);
        *score = self.config.min_score;
        
        Ok(())
    }
    
    /// Unban a peer
    pub async fn unban_peer(&self, peer_id: PeerId) -> crate::NetworkResult<()> {
        {
            let mut banned = self.banned_peers.write().await;
            banned.remove(&peer_id);
        }
        
        // Reset reputation to default
        let mut scores = self.scores.write().await;
        if let Some(context_scores) = scores.get_mut(&peer_id) {
            context_scores.insert(ReputationContext::Networking, self.config.default_score);
        }
        
        Ok(())
    }
    
    /// Get a peer's reputation score for a specific context
    pub async fn get_reputation_async(&self, peer_id: &PeerId, context: &ReputationContext) -> i32 {
        let scores = self.scores.read().await;
        scores
            .get(peer_id)
            .and_then(|contexts| contexts.get(context))
            .copied()
            .unwrap_or(self.config.default_score)
    }
    
    /// Get a peer's reputation score for a specific context (synchronous version)
    pub fn get_reputation(&self, peer_id: &PeerId, context: &ReputationContext) -> i32 {
        // This is a fallback for synchronous code paths that can't use the async version
        // In production, you should prefer the async version above
        if let Ok(scores) = self.scores.try_read() {
            scores
                .get(peer_id)
                .and_then(|contexts| contexts.get(context))
                .copied()
                .unwrap_or(self.config.default_score)
        } else {
            self.config.default_score
        }
    }
    
    /// Check if a peer is trusted asynchronously
    pub async fn is_trusted_async(&self, peer_id: &PeerId) -> bool {
        self.get_reputation_async(peer_id, &ReputationContext::Networking).await >= self.config.trusted_threshold
    }
    
    /// Check if a peer is trusted
    pub fn is_trusted(&self, peer_id: &PeerId) -> bool {
        self.get_reputation(peer_id, &ReputationContext::Networking) >= self.config.trusted_threshold
    }
    
    /// Check if a peer is banned asynchronously
    pub async fn is_banned_async(&self, peer_id: &PeerId) -> bool {
        let banned = self.banned_peers.read().await;
        if banned.contains_key(peer_id) {
            return true;
        }
        
        self.get_reputation_async(peer_id, &ReputationContext::Networking).await <= self.config.ban_threshold
    }
    
    /// Check if a peer is banned
    pub fn is_banned(&self, peer_id: &PeerId) -> bool {
        if let Ok(banned) = self.banned_peers.try_read() {
            if banned.contains_key(peer_id) {
                return true;
            }
        }
        
        self.get_reputation(peer_id, &ReputationContext::Networking) <= self.config.ban_threshold
    }
    
    /// Record response time from a peer
    pub async fn record_response_time(&self, peer_id: &PeerId, latency: Duration) -> crate::NetworkResult<()> {
        // Use response time to adjust reputation
        let latency_ms = latency.as_millis() as u64;
        
        // Interpret latency: low latency is good, high latency is bad
        let value = if latency_ms < 100 {
            InteractionValue::Positive(1)
        } else if latency_ms > 1000 {
            InteractionValue::Negative(1)
        } else {
            InteractionValue::Neutral
        };
        
        self.update_reputation(peer_id, ReputationContext::Networking, value).await
    }
    
    /// Start a task to periodically decay reputation scores
    pub async fn start_decay_task(&self) -> crate::NetworkResult<()> {
        // Clone what we need to pass to the task
        let scores = self.scores.clone();
        let decay_rate = self.config.decay_rate;
        
        // Create a task that decays scores every hour
        tokio::spawn(async move {
            let decay_interval = Duration::from_secs(3600); // 1 hour
            let mut interval = tokio::time::interval(decay_interval);
            
            loop {
                interval.tick().await;
                
                let mut scores_lock = scores.write().await;
                for (_peer_id, context_scores) in scores_lock.iter_mut() {
                    for (_context, score) in context_scores.iter_mut() {
                        // Apply decay - move score closer to 0
                        if *score > 0 {
                            *score = (*score as f32 * (1.0 - decay_rate)) as i32;
                        } else if *score < 0 {
                            *score = (*score as f32 * (1.0 - decay_rate)) as i32;
                        }
                    }
                }
            }
        });
        
        Ok(())
    }
}

/// Alias for ReputationChange for backward compatibility
pub type ReputationChange = InteractionValue; 