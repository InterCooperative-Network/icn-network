//! Peer reputation management system for the InterCooperative Network
//! 
//! This module implements a reputation system that tracks peer behavior and assigns
//! reputation scores based on their actions. The system helps make better decisions
//! about which peers to connect to, prioritize messages from, or avoid entirely.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use libp2p::PeerId;

use crate::NetworkError;

/// Reputation change values for different events
#[derive(Debug, Clone, Copy)]
pub enum ReputationChange {
    /// Message processed successfully
    MessageSuccess = 1,
    /// Message processing failed
    MessageFailure = -2,
    /// Invalid message format
    InvalidMessage = -5,
    /// Peer responded quickly
    FastResponse = 2,
    /// Peer response was slow
    SlowResponse = -1,
    /// Peer provided useful data
    UsefulData = 5,
    /// Peer provided invalid data
    InvalidData = -10,
    /// Peer violated protocol rules
    ProtocolViolation = -20,
    /// Peer helped with relay
    RelaySuccess = 3,
    /// Peer failed relay attempt
    RelayFailure = -3,
    /// Peer provided resources
    ResourceProvision = 4,
    /// Peer consumed excessive resources
    ResourceAbuse = -15,
}

/// Peer behavior categories
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BehaviorCategory {
    /// Message handling performance
    Messaging,
    /// Data validation and provision
    DataQuality,
    /// Protocol compliance
    ProtocolCompliance,
    /// Resource usage
    ResourceUsage,
    /// Network relay performance
    RelayPerformance,
}

/// Peer status based on reputation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerStatus {
    /// Trusted peer with high reputation
    Trusted,
    /// Normal peer with acceptable reputation
    Normal,
    /// Probation peer with low reputation
    Probation,
    /// Banned peer with very low reputation
    Banned,
}

impl PeerStatus {
    /// Get the reputation threshold for this status
    pub fn threshold(&self) -> i32 {
        match self {
            PeerStatus::Trusted => 50,
            PeerStatus::Normal => 0,
            PeerStatus::Probation => -25,
            PeerStatus::Banned => -50,
        }
    }
    
    /// Get the status for a given reputation score
    pub fn from_score(score: i32) -> Self {
        if score >= PeerStatus::Trusted.threshold() {
            PeerStatus::Trusted
        } else if score >= PeerStatus::Normal.threshold() {
            PeerStatus::Normal
        } else if score >= PeerStatus::Probation.threshold() {
            PeerStatus::Probation
        } else {
            PeerStatus::Banned
        }
    }
}

/// Detailed peer behavior tracking
#[derive(Debug, Clone)]
struct PeerBehavior {
    /// Reputation score per category
    category_scores: HashMap<BehaviorCategory, i32>,
    /// Last update time per category
    last_updates: HashMap<BehaviorCategory, Instant>,
    /// Number of violations per category
    violations: HashMap<BehaviorCategory, u32>,
    /// Time when peer was last banned
    last_banned: Option<Instant>,
    /// Number of times peer has been banned
    ban_count: u32,
    /// Recent behavior timestamps for analysis
    recent_events: Vec<(Instant, ReputationChange)>,
    /// Current peer group
    current_group: Option<String>,
    /// Time spent in current group
    group_time: Duration,
}

impl Default for PeerBehavior {
    fn default() -> Self {
        let mut category_scores = HashMap::new();
        let mut last_updates = HashMap::new();
        let mut violations = HashMap::new();
        
        // Initialize all categories
        for category in [
            BehaviorCategory::Messaging,
            BehaviorCategory::DataQuality,
            BehaviorCategory::ProtocolCompliance,
            BehaviorCategory::ResourceUsage,
            BehaviorCategory::RelayPerformance,
        ] {
            category_scores.insert(category, 0);
            last_updates.insert(category, Instant::now());
            violations.insert(category, 0);
        }
        
        Self {
            category_scores,
            last_updates,
            violations,
            last_banned: None,
            ban_count: 0,
            recent_events: Vec::new(),
            current_group: None,
            group_time: Duration::from_secs(0),
        }
    }
}

/// Configuration for reputation management
#[derive(Debug, Clone)]
pub struct ReputationConfig {
    /// Minimum reputation score (default: -100)
    pub min_score: i32,
    /// Maximum reputation score (default: 100)
    pub max_score: i32,
    /// Score decay rate per hour (default: 1)
    pub decay_rate: i32,
    /// Maximum violations before auto-ban (default: 5)
    pub max_violations: u32,
    /// Ban duration in hours (default: 24)
    pub ban_duration: Duration,
    /// Maximum ban count before permanent ban (default: 3)
    pub max_ban_count: u32,
    /// Whether to enable automatic peer banning (default: true)
    pub enable_auto_ban: bool,
    /// Category weights for scoring
    pub category_weights: CategoryWeights,
    /// Time window for recent behavior analysis (default: 24 hours)
    pub recent_window: Duration,
    /// Score boost for consistent good behavior (default: 1.1)
    pub consistency_multiplier: f32,
    /// Score penalty for repeated violations (default: 1.5)
    pub violation_multiplier: f32,
    /// Peer groups configuration
    pub peer_groups: Vec<PeerGroup>,
}

impl Default for ReputationConfig {
    fn default() -> Self {
        Self {
            min_score: -100,
            max_score: 100,
            decay_rate: 1,
            max_violations: 5,
            ban_duration: Duration::from_secs(24 * 3600),
            max_ban_count: 3,
            enable_auto_ban: true,
            category_weights: CategoryWeights::default(),
            recent_window: Duration::from_secs(24 * 3600),
            consistency_multiplier: 1.1,
            violation_multiplier: 1.5,
            peer_groups: Vec::new(),
        }
    }
}

/// Weight factors for different behavior categories
#[derive(Debug, Clone, Copy)]
pub struct CategoryWeights {
    /// Weight for messaging performance (default: 1.0)
    pub messaging: f32,
    /// Weight for data quality (default: 1.2)
    pub data_quality: f32,
    /// Weight for protocol compliance (default: 1.5)
    pub protocol_compliance: f32,
    /// Weight for resource usage (default: 1.3)
    pub resource_usage: f32,
    /// Weight for relay performance (default: 1.1)
    pub relay_performance: f32,
}

impl Default for CategoryWeights {
    fn default() -> Self {
        Self {
            messaging: 1.0,
            data_quality: 1.2,
            protocol_compliance: 1.5,
            resource_usage: 1.3,
            relay_performance: 1.1,
        }
    }
}

impl CategoryWeights {
    /// Get weight for a specific category
    pub fn get_weight(&self, category: BehaviorCategory) -> f32 {
        match category {
            BehaviorCategory::Messaging => self.messaging,
            BehaviorCategory::DataQuality => self.data_quality,
            BehaviorCategory::ProtocolCompliance => self.protocol_compliance,
            BehaviorCategory::ResourceUsage => self.resource_usage,
            BehaviorCategory::RelayPerformance => self.relay_performance,
        }
    }
}

/// Peer group for managing sets of peers with similar characteristics
#[derive(Debug, Clone)]
pub struct PeerGroup {
    /// Name of the group
    pub name: String,
    /// Minimum reputation score to join the group
    pub min_score: i32,
    /// Maximum number of peers in the group
    pub max_peers: usize,
    /// Priority level for message processing
    pub priority: u8,
    /// Additional privileges granted to group members
    pub privileges: Vec<String>,
}

/// Reputation manager for tracking peer behavior
pub struct ReputationManager {
    /// Configuration
    config: ReputationConfig,
    /// Peer behavior tracking
    behaviors: Arc<RwLock<HashMap<PeerId, PeerBehavior>>>,
    /// Banned peers
    banned: Arc<RwLock<HashMap<PeerId, Instant>>>,
}

impl ReputationManager {
    /// Create a new reputation manager
    pub fn new(config: ReputationConfig) -> Self {
        Self {
            config,
            behaviors: Arc::new(RwLock::new(HashMap::new())),
            banned: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Record a reputation change for a peer
    pub async fn record_change(
        &self,
        peer_id: PeerId,
        change: ReputationChange,
    ) -> Result<(), NetworkError> {
        let mut behaviors = self.behaviors.write().await;
        let behavior = behaviors.entry(peer_id).or_default();
        
        // Determine category and update score
        let (category, score_change) = match change {
            ReputationChange::MessageSuccess |
            ReputationChange::MessageFailure |
            ReputationChange::InvalidMessage => (BehaviorCategory::Messaging, change as i32),
            
            ReputationChange::FastResponse |
            ReputationChange::SlowResponse |
            ReputationChange::UsefulData |
            ReputationChange::InvalidData => (BehaviorCategory::DataQuality, change as i32),
            
            ReputationChange::ProtocolViolation => {
                let violations = behavior.violations.entry(BehaviorCategory::ProtocolCompliance)
                    .or_default();
                *violations += 1;
                
                if *violations >= self.config.max_violations && self.config.enable_auto_ban {
                    self.ban_peer(peer_id).await?;
                }
                
                (BehaviorCategory::ProtocolCompliance, change as i32)
            },
            
            ReputationChange::ResourceProvision |
            ReputationChange::ResourceAbuse => (BehaviorCategory::ResourceUsage, change as i32),
            
            ReputationChange::RelaySuccess |
            ReputationChange::RelayFailure => (BehaviorCategory::RelayPerformance, change as i32),
        };
        
        // Update category score
        let score = behavior.category_scores.entry(category).or_default();
        *score = (*score + score_change)
            .max(self.config.min_score)
            .min(self.config.max_score);
            
        // Update last update time
        behavior.last_updates.insert(category, Instant::now());
        
        // Check if peer should be banned based on total score
        let total_score: i32 = behavior.category_scores.values().sum();
        if total_score <= PeerStatus::Banned.threshold() && self.config.enable_auto_ban {
            self.ban_peer(peer_id).await?;
        }
        
        // Track recent behavior
        behavior.recent_events.push((Instant::now(), change));
        
        // Cleanup old events
        behavior.recent_events.retain(|(timestamp, _)| {
            timestamp.elapsed() <= self.config.recent_window
        });
        
        // Update peer group after reputation change
        self.update_peer_group(peer_id).await?;
        
        Ok(())
    }
    
    /// Ban a peer
    pub async fn ban_peer(&self, peer_id: PeerId) -> Result<(), NetworkError> {
        let mut behaviors = self.behaviors.write().await;
        let mut banned = self.banned.write().await;
        
        if let Some(behavior) = behaviors.get_mut(&peer_id) {
            behavior.ban_count += 1;
            behavior.last_banned = Some(Instant::now());
            
            // Check for permanent ban
            if behavior.ban_count >= self.config.max_ban_count {
                info!("Permanently banning peer {}", peer_id);
                banned.insert(peer_id, Instant::now());
            } else {
                info!("Temporarily banning peer {} for {:?}", peer_id, self.config.ban_duration);
                banned.insert(peer_id, Instant::now());
            }
        }
        
        Ok(())
    }
    
    /// Unban a peer if ban duration has expired
    pub async fn check_unban(&self, peer_id: PeerId) -> Result<bool, NetworkError> {
        let mut banned = self.banned.write().await;
        let behaviors = self.behaviors.read().await;
        
        if let Some(ban_time) = banned.get(&peer_id) {
            let behavior = behaviors.get(&peer_id)
                .ok_or_else(|| NetworkError::PeerNotFound)?;
                
            // Check if this is a permanent ban
            if behavior.ban_count >= self.config.max_ban_count {
                return Ok(false);
            }
            
            // Check if temporary ban has expired
            if ban_time.elapsed() >= self.config.ban_duration {
                banned.remove(&peer_id);
                info!("Unbanning peer {}", peer_id);
                return Ok(true);
            }
        }
        
        Ok(false)
    }
    
    /// Get a peer's current status
    pub async fn get_peer_status(&self, peer_id: PeerId) -> PeerStatus {
        let behaviors = self.behaviors.read().await;
        let banned = self.banned.read().await;
        
        // Check if peer is banned
        if banned.contains_key(&peer_id) {
            return PeerStatus::Banned;
        }
        
        // Calculate total reputation score
        if let Some(behavior) = behaviors.get(&peer_id) {
            let total_score: i32 = behavior.category_scores.values().sum();
            PeerStatus::from_score(total_score)
        } else {
            PeerStatus::Normal // New peers start with normal status
        }
    }
    
    /// Get detailed behavior statistics for a peer
    pub async fn get_peer_stats(&self, peer_id: PeerId) -> Option<PeerStats> {
        let behaviors = self.behaviors.read().await;
        let banned = self.banned.read().await;
        
        behaviors.get(&peer_id).map(|behavior| {
            let total_score: i32 = behavior.category_scores.values().sum();
            let total_violations: u32 = behavior.violations.values().sum();
            
            PeerStats {
                status: if banned.contains_key(&peer_id) {
                    PeerStatus::Banned
                } else {
                    PeerStatus::from_score(total_score)
                },
                total_score,
                category_scores: behavior.category_scores.clone(),
                total_violations,
                violations: behavior.violations.clone(),
                ban_count: behavior.ban_count,
                last_banned: behavior.last_banned,
            }
        })
    }
    
    /// Decay reputation scores for inactive peers
    pub async fn decay_scores(&self) {
        let mut behaviors = self.behaviors.write().await;
        let now = Instant::now();
        
        for behavior in behaviors.values_mut() {
            for (category, last_update) in &behavior.last_updates {
                let hours_elapsed = last_update.elapsed().as_secs() as f64 / 3600.0;
                let decay = (hours_elapsed * self.config.decay_rate as f64) as i32;
                
                if decay > 0 {
                    let score = behavior.category_scores.get_mut(category).unwrap();
                    *score = (*score - decay).max(self.config.min_score);
                }
            }
        }
    }

    /// Calculate weighted reputation score for a peer
    pub async fn calculate_weighted_score(&self, peer_id: PeerId) -> Option<f32> {
        let behaviors = self.behaviors.read().await;
        
        behaviors.get(&peer_id).map(|behavior| {
            let mut weighted_score = 0.0;
            
            for (category, score) in &behavior.category_scores {
                let weight = self.config.category_weights.get_weight(*category);
                weighted_score += *score as f32 * weight;
            }
            
            // Apply consistency multiplier for good behavior
            if behavior.violations.values().sum::<u32>() == 0 {
                weighted_score *= self.config.consistency_multiplier;
            }
            
            // Apply violation penalty for repeat offenders
            let total_violations: u32 = behavior.violations.values().sum();
            if total_violations > 0 {
                weighted_score /= (total_violations as f32 * self.config.violation_multiplier);
            }
            
            weighted_score
        })
    }

    /// Update peer group membership
    pub async fn update_peer_group(&self, peer_id: PeerId) -> Result<(), NetworkError> {
        let mut behaviors = self.behaviors.write().await;
        
        if let Some(behavior) = behaviors.get_mut(&peer_id) {
            let weighted_score = self.calculate_weighted_score(peer_id).await.unwrap_or(0.0);
            
            // Find the highest priority group the peer qualifies for
            let new_group = self.config.peer_groups.iter()
                .filter(|group| {
                    weighted_score >= group.min_score as f32 &&
                    self.count_group_members(&group.name).await <= group.max_peers
                })
                .max_by_key(|group| group.priority)
                .map(|group| group.name.clone());
                
            // Update group membership if changed
            if behavior.current_group != new_group {
                if let Some(old_group) = &behavior.current_group {
                    debug!("Peer {} leaving group {}", peer_id, old_group);
                }
                if let Some(new_group) = &new_group {
                    debug!("Peer {} joining group {}", peer_id, new_group);
                }
                
                behavior.current_group = new_group;
                behavior.group_time = Duration::from_secs(0);
            }
        }
        
        Ok(())
    }

    /// Count number of peers in a group
    async fn count_group_members(&self, group_name: &str) -> usize {
        let behaviors = self.behaviors.read().await;
        
        behaviors.values()
            .filter(|behavior| {
                behavior.current_group.as_ref().map(|g| g == group_name).unwrap_or(false)
            })
            .count()
    }

    /// Get peers in a specific group
    pub async fn get_group_peers(&self, group_name: &str) -> Vec<PeerId> {
        let behaviors = self.behaviors.read().await;
        
        behaviors.iter()
            .filter(|(_, behavior)| {
                behavior.current_group.as_ref().map(|g| g == group_name).unwrap_or(false)
            })
            .map(|(peer_id, _)| *peer_id)
            .collect()
    }

    /// Analyze recent behavior patterns
    pub async fn analyze_recent_behavior(&self, peer_id: PeerId) -> Option<BehaviorAnalysis> {
        let behaviors = self.behaviors.read().await;
        
        behaviors.get(&peer_id).map(|behavior| {
            let now = Instant::now();
            let recent_window = self.config.recent_window;
            
            // Filter recent events within the time window
            let recent_events: Vec<_> = behavior.recent_events.iter()
                .filter(|(timestamp, _)| timestamp.elapsed() <= recent_window)
                .collect();
                
            // Calculate statistics
            let total_events = recent_events.len();
            let positive_events = recent_events.iter()
                .filter(|(_, change)| *change as i32 > 0)
                .count();
            let negative_events = recent_events.iter()
                .filter(|(_, change)| *change as i32 < 0)
                .count();
                
            BehaviorAnalysis {
                total_events,
                positive_events,
                negative_events,
                event_rate: total_events as f32 / recent_window.as_secs_f32(),
                positive_ratio: if total_events > 0 {
                    positive_events as f32 / total_events as f32
                } else {
                    0.0
                },
            }
        })
    }
}

/// Detailed peer statistics
#[derive(Debug, Clone)]
pub struct PeerStats {
    /// Current peer status
    pub status: PeerStatus,
    /// Total reputation score
    pub total_score: i32,
    /// Scores per behavior category
    pub category_scores: HashMap<BehaviorCategory, i32>,
    /// Total number of violations
    pub total_violations: u32,
    /// Violations per category
    pub violations: HashMap<BehaviorCategory, u32>,
    /// Number of times peer has been banned
    pub ban_count: u32,
    /// Time of last ban
    pub last_banned: Option<Instant>,
}

/// Recent behavior analysis results
#[derive(Debug, Clone)]
pub struct BehaviorAnalysis {
    /// Total number of events in the analysis window
    pub total_events: usize,
    /// Number of positive reputation changes
    pub positive_events: usize,
    /// Number of negative reputation changes
    pub negative_events: usize,
    /// Rate of events per second
    pub event_rate: f32,
    /// Ratio of positive to total events
    pub positive_ratio: f32,
}

impl Clone for ReputationManager {
    fn clone(&self) -> Self {
        Self {
            reputations: self.reputations.clone(),
            config: self.config.clone(),
            command_tx: self.command_tx.clone(),
            task_handle: RwLock::new(None), // Don't clone the task handle
            storage: self.storage.clone(),
            metrics: self.metrics.clone(),
            command_senders: RwLock::new(Vec::new()), // Don't clone command senders
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::MockStorage;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_reputation_changes() {
        let config = ReputationConfig::default();
        let manager = ReputationManager::new(config, None, None).await.unwrap();
        let peer_id = PeerId::random();
        
        // Record some changes
        let score1 = manager.record_change(peer_id, ReputationChange::ConnectionEstablished).await.unwrap();
        assert_eq!(score1, 10);
        
        let score2 = manager.record_change(peer_id, ReputationChange::MessageSuccess).await.unwrap();
        assert_eq!(score2, 15);
        
        let score3 = manager.record_change(peer_id, ReputationChange::InvalidMessage).await.unwrap();
        assert_eq!(score3, -5);
        
        // Get the reputation and check it
        let rep = manager.get_reputation_value(peer_id).await.unwrap();
        assert_eq!(rep, -5);
    }
    
    #[tokio::test]
    async fn test_ban_unban() {
        let config = ReputationConfig::default();
        let manager = ReputationManager::new(config, None, None).await.unwrap();
        let peer_id = PeerId::random();
        
        // Check initial state
        assert!(!manager.is_banned(peer_id).await);
        
        // Ban the peer
        manager.ban_peer(peer_id).await.unwrap();
        assert!(manager.is_banned(peer_id).await);
        
        // Unban the peer
        manager.unban_peer(peer_id).await.unwrap();
        assert!(!manager.is_banned(peer_id).await);
        
        // Check score was reset to 0
        let rep = manager.get_reputation_value(peer_id).await.unwrap();
        assert_eq!(rep, 0);
    }
    
    #[tokio::test]
    async fn test_automatic_ban() {
        let config = ReputationConfig {
            ban_threshold: -30,
            ..Default::default()
        };
        let manager = ReputationManager::new(config, None, None).await.unwrap();
        let peer_id = PeerId::random();
        
        // Record changes until ban
        let score1 = manager.record_change(peer_id, ReputationChange::InvalidMessage).await.unwrap(); // -20
        assert_eq!(score1, -20);
        assert!(!manager.is_banned(peer_id).await);
        
        let score2 = manager.record_change(peer_id, ReputationChange::MessageFailure).await.unwrap(); // -10
        assert_eq!(score2, -30);
        assert!(manager.is_banned(peer_id).await);
    }
    
    #[tokio::test]
    async fn test_response_time() {
        let config = ReputationConfig {
            fast_response_threshold: 50,
            slow_response_threshold: 200,
            ..Default::default()
        };
        let manager = ReputationManager::new(config, None, None).await.unwrap();
        let peer_id = PeerId::random();
        
        // Fast response
        manager.record_response_time(peer_id, Duration::from_millis(30)).await.unwrap();
        let rep1 = manager.get_reputation(peer_id).await.unwrap();
        assert_eq!(rep1.response_times.len(), 1);
        assert_eq!(rep1.score, 1); // FastResponse gives +1
        
        // Slow response
        manager.record_response_time(peer_id, Duration::from_millis(300)).await.unwrap();
        let rep2 = manager.get_reputation(peer_id).await.unwrap();
        // Weighted average: (30*9 + 300)/10 = 57
        assert_eq!(rep2.response_times.len(), 2);
        assert_eq!(rep2.score, -1); // SlowResponse gives -2 after +1
    }
    
    #[tokio::test]
    async fn test_reputation_decay() {
        let config = ReputationConfig {
            decay_interval: Duration::from_secs(1),
            decay_factor: 0.5, // 50% decay every interval
            ..Default::default()
        };
        let manager = ReputationManager::new(config, None, None).await.unwrap();
        
        // Start the manager
        manager.start().await.unwrap();
        let peer_id = PeerId::random();
        
        // Set a high score
        manager.record_change(peer_id, ReputationChange::Manual(100)).await.unwrap();
        
        // Sleep to allow decay to happen
        tokio::time::sleep(Duration::from_secs(3)).await;
        
        // Check that score has decayed
        let rep = manager.get_reputation(peer_id).await.unwrap();
        assert!(rep < 100);
        
        // Stop the manager
        manager.stop().await.unwrap();
    }
    
    #[tokio::test]
    async fn test_persistence() {
        // Create a mock storage
        let storage = Arc::new(crate::tests::MockStorage::new());
        
        // Create config with storage key
        let config = ReputationConfig {
            storage_key: "test_reputation".to_string(),
            ..Default::default()
        };
        
        // Create and start a manager
        let manager1 = ReputationManager::new(config.clone(), Some(storage.clone()), None).await.unwrap();
        manager1.start().await.unwrap();
        
        // Add some reputation data
        let peer_id = PeerId::random();
        manager1.record_change(peer_id, ReputationChange::ConnectionEstablished).await.unwrap();
        manager1.save().await.unwrap();
        
        // Stop the first manager
        manager1.stop().await.unwrap();
        
        // Create a new manager with the same storage
        let manager2 = ReputationManager::new(config, Some(storage), None).await.unwrap();
        manager2.start().await.unwrap();
        
        // Check that data was loaded
        let rep = manager2.get_reputation_value(peer_id).await.unwrap();
        assert_eq!(rep, 10);
        
        manager2.stop().await.unwrap();
    }
} 