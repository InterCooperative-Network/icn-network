//! Peer reputation management system for the InterCooperative Network
//! 
//! This module implements a reputation system that tracks peer behavior and assigns
//! reputation scores based on their actions. The system helps make better decisions
//! about which peers to connect to, prioritize messages from, or avoid entirely.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use futures::StreamExt;
use libp2p::PeerId;
use log::{debug, info, warn};
use serde::{Serialize, Deserialize};
use tokio::sync::{RwLock, mpsc};
use tokio::task::JoinHandle;

use crate::error::{NetworkResult, NetworkError};
use crate::metrics::NetworkMetrics;
use crate::storage::Storage;

/// Maximum number of reputation history items to store per peer
const MAX_HISTORY_ITEMS: usize = 100;

/// Types of actions that affect a peer's reputation
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ReputationChange {
    /// Successful connection established (+10)
    ConnectionEstablished,
    /// Connection dropped unexpectedly (-5)
    ConnectionLost,
    /// Successful message exchange (+5)
    MessageSuccess,
    /// Failed message exchange (-10)
    MessageFailure,
    /// Invalid or malformed message (-20)
    InvalidMessage,
    /// Successfully verified message (+15)
    VerifiedMessage,
    /// Helped with peer discovery (+5)
    DiscoveryHelp,
    /// Deliberately provided incorrect information (-50)
    Misinformation,
    /// Explicit ban by user or administrator (-100)
    ExplicitBan,
    /// Failed to respond in a timely manner (-2)
    SlowResponse,
    /// Fast response (+1)
    FastResponse,
    /// Explicit administrative unban (+0) (just resets to 0)
    AdminUnban,
    /// Manual value adjustment (used for testing or special cases)
    Manual(i32),
}

impl ReputationChange {
    /// Get the score value for this reputation change
    pub fn value(&self) -> i32 {
        match self {
            Self::ConnectionEstablished => 10,
            Self::ConnectionLost => -5,
            Self::MessageSuccess => 5,
            Self::MessageFailure => -10,
            Self::InvalidMessage => -20,
            Self::VerifiedMessage => 15,
            Self::DiscoveryHelp => 5,
            Self::Misinformation => -50,
            Self::ExplicitBan => -100,
            Self::SlowResponse => -2,
            Self::FastResponse => 1,
            Self::AdminUnban => 0, // This resets to 0, not an increment
            Self::Manual(val) => *val,
        }
    }
    
    /// Determine if this change is a reset type (like AdminUnban)
    pub fn is_reset(&self) -> bool {
        matches!(self, Self::AdminUnban)
    }
    
    /// Check if this is a positive change
    pub fn is_positive(&self) -> bool {
        self.value() > 0
    }
    
    /// Check if this is a negative change
    pub fn is_negative(&self) -> bool {
        self.value() < 0
    }
}

/// History item for reputation changes
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ReputationHistoryItem {
    /// The type of change
    change: ReputationChange,
    /// When the change occurred
    timestamp: u64,
    /// The value of the change
    value: i32,
    /// Score after the change
    score_after: i32,
}

/// Information about a peer's reputation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerReputation {
    /// Current reputation score
    score: i32,
    /// Timestamp of last update
    last_update: u64,
    /// Is this peer explicitly banned
    is_banned: bool,
    /// History of recent reputation changes
    history: VecDeque<ReputationHistoryItem>,
    /// Count of positive interactions
    positive_count: u32,
    /// Count of negative interactions
    negative_count: u32,
    /// First seen timestamp
    first_seen: u64,
    /// Last seen timestamp
    last_seen: u64,
    /// Average response time in milliseconds
    avg_response_time: Option<u64>,
}

impl Default for PeerReputation {
    fn default() -> Self {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
            
        Self {
            score: 0,
            last_update: now,
            is_banned: false,
            history: VecDeque::with_capacity(MAX_HISTORY_ITEMS),
            positive_count: 0,
            negative_count: 0,
            first_seen: now,
            last_seen: now,
            avg_response_time: None,
        }
    }
}

impl PeerReputation {
    /// Create a new peer reputation
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Apply a reputation change
    pub fn apply_change(&mut self, change: ReputationChange) -> i32 {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        self.last_seen = now;
        
        // For reset changes, set the score directly
        if change.is_reset() {
            self.score = 0;
        } else {
            // Otherwise apply the change value
            self.score += change.value();
        }
        
        // Update positive/negative counts
        if change.is_positive() {
            self.positive_count += 1;
        } else if change.is_negative() {
            self.negative_count += 1;
        }
        
        // Record in history
        if self.history.len() >= MAX_HISTORY_ITEMS {
            self.history.pop_front();
        }
        
        self.history.push_back(ReputationHistoryItem {
            change,
            timestamp: now,
            value: change.value(),
            score_after: self.score,
        });
        
        self.last_update = now;
        
        self.score
    }
    
    /// Update response time
    pub fn update_response_time(&mut self, time_ms: u64) {
        // If first time, just set it
        if let Some(avg) = self.avg_response_time {
            // Otherwise do a weighted average (90% old, 10% new)
            self.avg_response_time = Some((avg * 9 + time_ms) / 10);
        } else {
            self.avg_response_time = Some(time_ms);
        }
    }
    
    /// Mark this peer as banned
    pub fn ban(&mut self) {
        self.is_banned = true;
    }
    
    /// Unban this peer
    pub fn unban(&mut self) {
        self.is_banned = false;
    }
    
    /// Get the current score
    pub fn score(&self) -> i32 {
        self.score
    }
    
    /// Check if this peer is banned
    pub fn is_banned(&self) -> bool {
        self.is_banned
    }
    
    /// Get history of reputation changes
    pub fn history(&self) -> &VecDeque<ReputationHistoryItem> {
        &self.history
    }
    
    /// Get the positive interactions count
    pub fn positive_count(&self) -> u32 {
        self.positive_count
    }
    
    /// Get the negative interactions count
    pub fn negative_count(&self) -> u32 {
        self.negative_count
    }
    
    /// Get the response time
    pub fn avg_response_time(&self) -> Option<u64> {
        self.avg_response_time
    }
    
    /// Apply decay to the reputation score
    pub fn apply_decay(&mut self, decay_factor: f64, decay_interval: Duration) {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
            
        // Only decay if some time has passed
        let seconds_since_update = now.saturating_sub(self.last_update);
        if seconds_since_update > 0 {
            let intervals = seconds_since_update as f64 / decay_interval.as_secs() as f64;
            
            // If intervals is very small (< 0.001), skip decay
            if intervals > 0.001 {
                // Apply decay - move score toward 0
                if self.score > 0 {
                    let decay = (self.score as f64 * decay_factor * intervals).ceil() as i32;
                    self.score = self.score.saturating_sub(decay.min(self.score));
                } else if self.score < 0 {
                    let decay = (self.score.abs() as f64 * decay_factor * intervals).ceil() as i32;
                    self.score = self.score.saturating_add(decay.min(self.score.abs()));
                }
                
                self.last_update = now;
            }
        }
    }
}

/// Configuration for the reputation manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationConfig {
    /// Threshold for banning peers
    pub ban_threshold: i32,
    /// Decay factor for reputation (per decay interval)
    pub decay_factor: f64,
    /// Interval for reputation decay
    pub decay_interval: Duration,
    /// Threshold for considering a peer "good"
    pub good_threshold: i32,
    /// Path to reputation storage
    pub storage_path: Option<String>,
    /// Response time threshold for fast response in milliseconds
    pub fast_response_threshold: u64,
    /// Response time threshold for slow response in milliseconds
    pub slow_response_threshold: u64,
    /// Persistence interval for saving reputation data
    pub persistence_interval: Duration,
}

impl Default for ReputationConfig {
    fn default() -> Self {
        Self {
            ban_threshold: -50,
            decay_factor: 0.05,
            decay_interval: Duration::from_secs(3600), // 1 hour
            good_threshold: 25,
            storage_path: Some(".peer_reputation".to_string()),
            fast_response_threshold: 100, // 100ms
            slow_response_threshold: 1000, // 1000ms
            persistence_interval: Duration::from_secs(300), // 5 minutes
        }
    }
}

/// Command for the reputation manager
#[derive(Debug)]
enum ReputationCommand {
    /// Record a change to a peer's reputation
    RecordChange(PeerId, ReputationChange, tokio::sync::oneshot::Sender<NetworkResult<i32>>),
    /// Record a response time for a peer
    RecordResponseTime(PeerId, Duration, tokio::sync::oneshot::Sender<NetworkResult<()>>),
    /// Check if a peer is banned
    IsBanned(PeerId, tokio::sync::oneshot::Sender<bool>),
    /// Ban a peer
    BanPeer(PeerId, tokio::sync::oneshot::Sender<NetworkResult<()>>),
    /// Unban a peer
    UnbanPeer(PeerId, tokio::sync::oneshot::Sender<NetworkResult<()>>),
    /// Get a peer's reputation
    GetReputation(PeerId, tokio::sync::oneshot::Sender<Option<PeerReputation>>),
    /// Save reputation data to storage
    Save(tokio::sync::oneshot::Sender<NetworkResult<()>>),
    /// Stop the reputation manager
    Stop(tokio::sync::oneshot::Sender<NetworkResult<()>>),
}

/// Manager for peer reputations
pub struct ReputationManager {
    /// Peer reputation data
    reputations: Arc<RwLock<HashMap<PeerId, PeerReputation>>>,
    /// Configuration for the reputation system
    config: ReputationConfig,
    /// Command sender for the background task
    command_tx: mpsc::Sender<ReputationCommand>,
    /// Handle for the background task
    task_handle: RwLock<Option<JoinHandle<()>>>,
    /// Storage provider
    storage: Option<Arc<dyn Storage>>,
    /// Metrics
    metrics: Option<NetworkMetrics>,
}

impl ReputationManager {
    /// Create a new reputation manager
    pub fn new(
        config: ReputationConfig,
        storage: Option<Arc<dyn Storage>>,
        metrics: Option<NetworkMetrics>,
    ) -> Self {
        let (command_tx, command_rx) = mpsc::channel(32);
        
        Self {
            reputations: Arc::new(RwLock::new(HashMap::new())),
            config,
            command_tx,
            task_handle: RwLock::new(None),
            storage,
            metrics,
        }
    }
    
    /// Start the reputation manager
    pub async fn start(&self) -> NetworkResult<()> {
        // Load stored reputation data if available
        if let Some(storage) = &self.storage {
            if let Some(path) = &self.config.storage_path {
                if let Ok(data) = storage.get(path.as_bytes()).await {
                    if !data.is_empty() {
                        if let Ok(rep_data) = serde_json::from_slice::<HashMap<String, PeerReputation>>(&data) {
                            let mut reputations = self.reputations.write().await;
                            for (peer_id_str, reputation) in rep_data {
                                if let Ok(peer_id) = PeerId::from_bytes(bs58::decode(&peer_id_str).into_vec().map_err(|_| NetworkError::DecodingError)?) {
                                    reputations.insert(peer_id, reputation);
                                    
                                    // Update metrics
                                    if let Some(metrics) = &self.metrics {
                                        metrics.update_reputation_score(&peer_id.to_string(), reputation.score());
                                        if reputation.is_banned() {
                                            metrics.record_peer_banned(&peer_id.to_string());
                                        }
                                    }
                                }
                            }
                            
                            debug!("Loaded {} peer reputation records", reputations.len());
                        }
                    }
                }
            }
        }
        
        // Start the background task
        let task = self.start_background_task(command_rx);
        let mut handle = self.task_handle.write().await;
        *handle = Some(task);
        
        Ok(())
    }
    
    // Start the background task for handling commands and periodic tasks
    fn start_background_task(&self, mut command_rx: mpsc::Receiver<ReputationCommand>) -> JoinHandle<()> {
        let reputations = self.reputations.clone();
        let config = self.config.clone();
        let storage = self.storage.clone();
        let metrics = self.metrics.clone();
        
        tokio::spawn(async move {
            let mut decay_interval = tokio::time::interval(Duration::from_secs(60));
            let mut persistence_interval = tokio::time::interval(config.persistence_interval);
            
            loop {
                tokio::select! {
                    Some(cmd) = command_rx.recv() => {
                        match cmd {
                            ReputationCommand::RecordChange(peer_id, change, response_tx) => {
                                let score = Self::handle_record_change(
                                    &reputations, 
                                    &metrics, 
                                    &peer_id, 
                                    change, 
                                    &config
                                ).await;
                                
                                let _ = response_tx.send(Ok(score));
                            }
                            ReputationCommand::RecordResponseTime(peer_id, time, response_tx) => {
                                Self::handle_response_time(
                                    &reputations, 
                                    &metrics, 
                                    &peer_id, 
                                    time, 
                                    &config
                                ).await;
                                
                                let _ = response_tx.send(Ok(()));
                            }
                            ReputationCommand::IsBanned(peer_id, response_tx) => {
                                let is_banned = Self::handle_is_banned(&reputations, &peer_id).await;
                                let _ = response_tx.send(is_banned);
                            }
                            ReputationCommand::BanPeer(peer_id, response_tx) => {
                                Self::handle_ban_peer(&reputations, &metrics, &peer_id).await;
                                let _ = response_tx.send(Ok(()));
                            }
                            ReputationCommand::UnbanPeer(peer_id, response_tx) => {
                                Self::handle_unban_peer(&reputations, &metrics, &peer_id).await;
                                let _ = response_tx.send(Ok(()));
                            }
                            ReputationCommand::GetReputation(peer_id, response_tx) => {
                                let rep = Self::handle_get_reputation(&reputations, &peer_id).await;
                                let _ = response_tx.send(rep);
                            }
                            ReputationCommand::Save(response_tx) => {
                                let result = Self::handle_save(&reputations, &storage, &config).await;
                                let _ = response_tx.send(result);
                            }
                            ReputationCommand::Stop(response_tx) => {
                                // Save before stopping
                                let result = Self::handle_save(&reputations, &storage, &config).await;
                                let _ = response_tx.send(result);
                                break;
                            }
                        }
                    }
                    _ = decay_interval.tick() => {
                        Self::handle_decay(&reputations, &metrics, &config).await;
                    }
                    _ = persistence_interval.tick() => {
                        let _ = Self::handle_save(&reputations, &storage, &config).await;
                    }
                    else => break,
                }
            }
        })
    }
    
    // Handle recording a reputation change
    async fn handle_record_change(
        reputations: &RwLock<HashMap<PeerId, PeerReputation>>,
        metrics: &Option<NetworkMetrics>,
        peer_id: &PeerId,
        change: ReputationChange,
        config: &ReputationConfig,
    ) -> i32 {
        let mut reputations = reputations.write().await;
        let reputation = reputations.entry(peer_id.clone()).or_insert_with(PeerReputation::new);
        
        // Apply the change
        let new_score = reputation.apply_change(change);
        
        // Check if this change puts the peer below the ban threshold
        if new_score <= config.ban_threshold && !reputation.is_banned() {
            reputation.ban();
            
            // Record the ban in metrics
            if let Some(metrics) = metrics {
                metrics.record_peer_banned(&peer_id.to_string());
            }
            
            info!("Peer {} has been banned: score {} ≤ threshold {}", 
                peer_id, new_score, config.ban_threshold);
        }
        
        // Update metrics
        if let Some(metrics) = metrics {
            metrics.update_reputation_score(&peer_id.to_string(), new_score);
            
            // Record the type of change
            if change.is_positive() {
                metrics.record_reputation_change(&peer_id.to_string(), "positive");
            } else if change.is_negative() {
                metrics.record_reputation_change(&peer_id.to_string(), "negative");
            } else {
                metrics.record_reputation_change(&peer_id.to_string(), "neutral");
            }
        }
        
        new_score
    }
    
    // Handle recording a response time
    async fn handle_response_time(
        reputations: &RwLock<HashMap<PeerId, PeerReputation>>,
        metrics: &Option<NetworkMetrics>,
        peer_id: &PeerId,
        time: Duration,
        config: &ReputationConfig,
    ) {
        let mut reputations = reputations.write().await;
        let reputation = reputations.entry(peer_id.clone()).or_insert_with(PeerReputation::new);
        
        // Update the response time
        let time_ms = time.as_millis() as u64;
        reputation.update_response_time(time_ms);
        
        // Potentially apply a reputation change based on response time
        if time_ms <= config.fast_response_threshold {
            // Fast response, reward
            reputation.apply_change(ReputationChange::FastResponse);
            
            // Update metrics
            if let Some(metrics) = metrics {
                metrics.record_reputation_change(&peer_id.to_string(), "positive");
            }
        } else if time_ms >= config.slow_response_threshold {
            // Slow response, penalize
            reputation.apply_change(ReputationChange::SlowResponse);
            
            // Update metrics
            if let Some(metrics) = metrics {
                metrics.record_reputation_change(&peer_id.to_string(), "negative");
            }
        }
    }
    
    // Handle checking if a peer is banned
    async fn handle_is_banned(
        reputations: &RwLock<HashMap<PeerId, PeerReputation>>,
        peer_id: &PeerId,
    ) -> bool {
        let reputations = reputations.read().await;
        if let Some(reputation) = reputations.get(peer_id) {
            reputation.is_banned()
        } else {
            false
        }
    }
    
    // Handle banning a peer
    async fn handle_ban_peer(
        reputations: &RwLock<HashMap<PeerId, PeerReputation>>,
        metrics: &Option<NetworkMetrics>,
        peer_id: &PeerId,
    ) {
        let mut reputations = reputations.write().await;
        let reputation = reputations.entry(peer_id.clone()).or_insert_with(PeerReputation::new);
        
        // Ban the peer and apply the explicit ban change
        reputation.apply_change(ReputationChange::ExplicitBan);
        reputation.ban();
        
        // Update metrics
        if let Some(metrics) = metrics {
            metrics.update_reputation_score(&peer_id.to_string(), reputation.score());
            metrics.record_peer_banned(&peer_id.to_string());
        }
        
        info!("Peer {} has been explicitly banned", peer_id);
    }
    
    // Handle unbanning a peer
    async fn handle_unban_peer(
        reputations: &RwLock<HashMap<PeerId, PeerReputation>>,
        metrics: &Option<NetworkMetrics>,
        peer_id: &PeerId,
    ) {
        let mut reputations = reputations.write().await;
        let reputation = reputations.entry(peer_id.clone()).or_insert_with(PeerReputation::new);
        
        // Unban the peer and reset the score
        reputation.apply_change(ReputationChange::AdminUnban);
        reputation.unban();
        
        // Update metrics
        if let Some(metrics) = metrics {
            metrics.update_reputation_score(&peer_id.to_string(), reputation.score());
            metrics.record_peer_unbanned(&peer_id.to_string());
        }
        
        info!("Peer {} has been unbanned", peer_id);
    }
    
    // Handle getting a peer's reputation
    async fn handle_get_reputation(
        reputations: &RwLock<HashMap<PeerId, PeerReputation>>,
        peer_id: &PeerId,
    ) -> Option<PeerReputation> {
        let reputations = reputations.read().await;
        reputations.get(peer_id).cloned()
    }
    
    // Handle saving reputation data
    async fn handle_save(
        reputations: &RwLock<HashMap<PeerId, PeerReputation>>,
        storage: &Option<Arc<dyn Storage>>,
        config: &ReputationConfig,
    ) -> NetworkResult<()> {
        if let Some(storage) = storage {
            if let Some(path) = &config.storage_path {
                let reputations = reputations.read().await;
                
                // Convert to a string-keyed map for easier serialization
                let mut rep_data = HashMap::new();
                for (peer_id, reputation) in reputations.iter() {
                    rep_data.insert(peer_id.to_string(), reputation.clone());
                }
                
                let data = serde_json::to_vec(&rep_data).map_err(|_| NetworkError::EncodingError)?;
                
                storage.put(path.as_bytes(), &data).await?;
                debug!("Saved {} peer reputation records", reputations.len());
            }
        }
        
        Ok(())
    }
    
    // Handle reputation decay
    async fn handle_decay(
        reputations: &RwLock<HashMap<PeerId, PeerReputation>>,
        metrics: &Option<NetworkMetrics>,
        config: &ReputationConfig,
    ) {
        let mut reputations = reputations.write().await;
        let decay_count = reputations.len();
        
        for (peer_id, reputation) in reputations.iter_mut() {
            let old_score = reputation.score();
            reputation.apply_decay(config.decay_factor, config.decay_interval);
            
            // If score changed, update metrics
            if old_score != reputation.score() && metrics.is_some() {
                if let Some(m) = metrics {
                    m.update_reputation_score(&peer_id.to_string(), reputation.score());
                }
            }
        }
        
        // Record decay processing in metrics
        if let Some(m) = metrics {
            m.record_reputation_decay(decay_count);
        }
    }
    
    /// Record a change to a peer's reputation
    pub async fn record_change(&self, peer_id: &PeerId, change: ReputationChange) -> NetworkResult<i32> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        
        if let Err(e) = self.command_tx.send(ReputationCommand::RecordChange(
            peer_id.clone(), 
            change,
            tx
        )).await {
            return Err(NetworkError::Other(format!("Failed to send reputation command: {}", e)));
        }
        
        rx.await.map_err(|_| NetworkError::Other("Failed to receive response".to_string()))?
    }
    
    /// Record a response time from a peer
    pub async fn record_response_time(&self, peer_id: &PeerId, time: Duration) -> NetworkResult<()> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        
        if let Err(e) = self.command_tx.send(ReputationCommand::RecordResponseTime(
            peer_id.clone(), 
            time,
            tx
        )).await {
            return Err(NetworkError::Other(format!("Failed to send reputation command: {}", e)));
        }
        
        rx.await.map_err(|_| NetworkError::Other("Failed to receive response".to_string()))?
    }
    
    /// Check if a peer is banned
    pub async fn is_banned(&self, peer_id: &PeerId) -> bool {
        let (tx, rx) = tokio::sync::oneshot::channel();
        
        if let Err(_) = self.command_tx.send(ReputationCommand::IsBanned(
            peer_id.clone(), 
            tx
        )).await {
            // If we can't send the command, assume not banned (safer default)
            return false;
        }
        
        rx.await.unwrap_or(false)
    }
    
    /// Ban a peer
    pub async fn ban_peer(&self, peer_id: &PeerId) -> NetworkResult<()> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        
        if let Err(e) = self.command_tx.send(ReputationCommand::BanPeer(
            peer_id.clone(), 
            tx
        )).await {
            return Err(NetworkError::Other(format!("Failed to send reputation command: {}", e)));
        }
        
        rx.await.map_err(|_| NetworkError::Other("Failed to receive response".to_string()))?
    }
    
    /// Unban a peer
    pub async fn unban_peer(&self, peer_id: &PeerId) -> NetworkResult<()> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        
        if let Err(e) = self.command_tx.send(ReputationCommand::UnbanPeer(
            peer_id.clone(), 
            tx
        )).await {
            return Err(NetworkError::Other(format!("Failed to send reputation command: {}", e)));
        }
        
        rx.await.map_err(|_| NetworkError::Other("Failed to receive response".to_string()))?
    }
    
    /// Get a peer's reputation
    pub async fn get_reputation(&self, peer_id: &PeerId) -> Option<PeerReputation> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        
        if let Err(_) = self.command_tx.send(ReputationCommand::GetReputation(
            peer_id.clone(), 
            tx
        )).await {
            return None;
        }
        
        rx.await.unwrap_or(None)
    }
    
    /// Save reputation data
    pub async fn save(&self) -> NetworkResult<()> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        
        if let Err(e) = self.command_tx.send(ReputationCommand::Save(tx)).await {
            return Err(NetworkError::Other(format!("Failed to send reputation command: {}", e)));
        }
        
        rx.await.map_err(|_| NetworkError::Other("Failed to receive response".to_string()))?
    }
    
    /// Stop the reputation manager
    pub async fn stop(&self) -> NetworkResult<()> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        
        if let Err(e) = self.command_tx.send(ReputationCommand::Stop(tx)).await {
            return Err(NetworkError::Other(format!("Failed to send reputation command: {}", e)));
        }
        
        // Wait for the save to complete
        let result = rx.await.map_err(|_| NetworkError::Other("Failed to receive response".to_string()))?;
        
        // Clear the task handle
        let mut handle = self.task_handle.write().await;
        if let Some(task) = handle.take() {
            task.abort();
        }
        
        result
    }
    
    /// Get an immutable reference to the reputations
    pub async fn reputations(&self) -> tokio::sync::RwLockReadGuard<'_, HashMap<PeerId, PeerReputation>> {
        self.reputations.read().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::MockStorage;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_reputation_changes() {
        let config = ReputationConfig::default();
        let manager = ReputationManager::new(config, None, None);
        let peer_id = PeerId::random();
        
        // Record some changes
        let score1 = manager.record_change(&peer_id, ReputationChange::ConnectionEstablished).await.unwrap();
        assert_eq!(score1, 10);
        
        let score2 = manager.record_change(&peer_id, ReputationChange::MessageSuccess).await.unwrap();
        assert_eq!(score2, 15);
        
        let score3 = manager.record_change(&peer_id, ReputationChange::InvalidMessage).await.unwrap();
        assert_eq!(score3, -5);
        
        // Get the reputation and check it
        let rep = manager.get_reputation(&peer_id).await.unwrap();
        assert_eq!(rep.score(), -5);
        assert_eq!(rep.history().len(), 3);
    }
    
    #[tokio::test]
    async fn test_ban_unban() {
        let config = ReputationConfig::default();
        let manager = ReputationManager::new(config, None, None);
        let peer_id = PeerId::random();
        
        // Check initial state
        assert!(!manager.is_banned(&peer_id).await);
        
        // Ban the peer
        manager.ban_peer(&peer_id).await.unwrap();
        assert!(manager.is_banned(&peer_id).await);
        
        // Unban the peer
        manager.unban_peer(&peer_id).await.unwrap();
        assert!(!manager.is_banned(&peer_id).await);
        
        // Check score was reset to 0
        let rep = manager.get_reputation(&peer_id).await.unwrap();
        assert_eq!(rep.score(), 0);
    }
    
    #[tokio::test]
    async fn test_automatic_ban() {
        let config = ReputationConfig {
            ban_threshold: -30,
            ..Default::default()
        };
        let manager = ReputationManager::new(config, None, None);
        let peer_id = PeerId::random();
        
        // Record changes until ban
        let score1 = manager.record_change(&peer_id, ReputationChange::InvalidMessage).await.unwrap(); // -20
        assert_eq!(score1, -20);
        assert!(!manager.is_banned(&peer_id).await);
        
        let score2 = manager.record_change(&peer_id, ReputationChange::MessageFailure).await.unwrap(); // -10
        assert_eq!(score2, -30);
        assert!(manager.is_banned(&peer_id).await);
    }
    
    #[tokio::test]
    async fn test_response_time() {
        let config = ReputationConfig {
            fast_response_threshold: 50,
            slow_response_threshold: 200,
            ..Default::default()
        };
        let manager = ReputationManager::new(config, None, None);
        let peer_id = PeerId::random();
        
        // Fast response
        manager.record_response_time(&peer_id, Duration::from_millis(30)).await.unwrap();
        let rep1 = manager.get_reputation(&peer_id).await.unwrap();
        assert_eq!(rep1.avg_response_time(), Some(30));
        assert_eq!(rep1.score(), 1); // FastResponse gives +1
        
        // Slow response
        manager.record_response_time(&peer_id, Duration::from_millis(300)).await.unwrap();
        let rep2 = manager.get_reputation(&peer_id).await.unwrap();
        // Weighted average: (30*9 + 300)/10 = 57
        assert_eq!(rep2.avg_response_time(), Some(57));
        assert_eq!(rep2.score(), -1); // SlowResponse gives -2 after +1
    }
    
    #[tokio::test]
    async fn test_reputation_decay() {
        let config = ReputationConfig {
            decay_factor: 0.5,
            decay_interval: Duration::from_secs(1),
            ..Default::default()
        };
        let manager = ReputationManager::new(config, None, None);
        
        // Start the manager
        manager.start().await.unwrap();
        let peer_id = PeerId::random();
        
        // Set a high score
        manager.record_change(&peer_id, ReputationChange::Manual(100)).await.unwrap();
        
        // Sleep to allow decay to happen
        tokio::time::sleep(Duration::from_secs(3)).await;
        
        // Check that score has decayed
        let rep = manager.get_reputation(&peer_id).await.unwrap();
        assert!(rep.score() < 100);
        
        // Stop the manager
        manager.stop().await.unwrap();
    }
    
    #[tokio::test]
    async fn test_persistence() {
        // Create a mock storage
        let storage = Arc::new(MockStorage::new());
        
        let config = ReputationConfig {
            storage_path: Some("test_rep".to_string()),
            ..Default::default()
        };
        
        // Create and start a manager
        let manager1 = ReputationManager::new(config.clone(), Some(storage.clone()), None);
        manager1.start().await.unwrap();
        
        // Add some reputation data
        let peer_id = PeerId::random();
        manager1.record_change(&peer_id, ReputationChange::ConnectionEstablished).await.unwrap();
        manager1.save().await.unwrap();
        
        // Stop the manager
        manager1.stop().await.unwrap();
        
        // Create a new manager with the same storage
        let manager2 = ReputationManager::new(config, Some(storage), None);
        manager2.start().await.unwrap();
        
        // Check that data was loaded
        let rep = manager2.get_reputation(&peer_id).await.unwrap();
        assert_eq!(rep.score(), 10);
        
        manager2.stop().await.unwrap();
    }
} 