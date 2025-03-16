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
use tracing::{debug, info, warn, error};
use serde::{Serialize, Deserialize};
use tokio::sync::{RwLock, mpsc};
use tokio::task::JoinHandle;

use crate::{NetworkResult, NetworkError};
use crate::metrics::NetworkMetrics;
use icn_core::storage::Storage;

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
    /// Reputation score for this peer
    pub value: i32,
    /// Last time this peer was seen
    pub last_seen: SystemTime,
    /// Last time the reputation was updated
    pub last_updated: SystemTime,
    /// Is this peer explicitly banned
    pub banned: bool,
    /// When the peer was banned (if applicable)
    pub ban_time: Option<SystemTime>,
    /// History of response times (for calculating averages)
    pub response_times: Vec<Duration>,
    /// Average response time
    pub avg_response_time: Duration,
}

impl PeerReputation {
    /// Create a new peer reputation with default values
    pub fn new() -> Self {
        let now = SystemTime::now();
        Self {
            value: 0,
            last_seen: now,
            last_updated: now,
            banned: false,
            ban_time: None,
            response_times: Vec::new(),
            avg_response_time: Duration::from_millis(0),
        }
    }

    /// Record a change to the peer's reputation
    pub fn record_change(&mut self, change: ReputationChange) -> i32 {
        let score = self.value + change.value();
        self.value = score;
        score
    }
}

/// Configuration for the reputation system
#[derive(Debug, Clone)]
pub struct ReputationConfig {
    /// Minimum reputation value
    pub min_value: i32,
    /// Maximum reputation value
    pub max_value: i32,
    /// Threshold for banning peers
    pub ban_threshold: i32,
    /// Threshold for unbanning peers
    pub unban_threshold: i32,
    /// Threshold for considering a peer "good"
    pub good_threshold: i32,
    /// Decay factor (how quickly reputation returns to neutral)
    pub decay_factor: f64,
    /// Decay interval in seconds
    pub decay_interval: Duration,
    /// Storage key for saving reputation data
    pub storage_key: String,
    /// Value for connection established
    pub connection_established_value: i32,
    /// Value for connection closed
    pub connection_closed_value: i32,
    /// Value for successful message
    pub message_success_value: i32,
    /// Value for failed message
    pub message_failure_value: i32,
    /// Value for invalid message
    pub invalid_message_value: i32,
    /// Maximum number of response times to keep per peer
    pub max_response_times: usize,
    /// Threshold for fast response time in milliseconds
    pub fast_response_threshold: u64,
    /// Threshold for slow response time in milliseconds
    pub slow_response_threshold: u64,
}

impl Default for ReputationConfig {
    fn default() -> Self {
        Self {
            min_value: -100,
            max_value: 100,
            ban_threshold: -50,
            unban_threshold: -20,
            good_threshold: 20,
            decay_factor: 0.9,
            decay_interval: Duration::from_secs(3600), // 1 hour
            storage_key: "reputation".to_string(),
            connection_established_value: 1,
            connection_closed_value: 0,
            message_success_value: 1,
            message_failure_value: -1,
            invalid_message_value: -5,
            max_response_times: 10,
            fast_response_threshold: 100,
            slow_response_threshold: 1000,
        }
    }
}

/// Commands that can be sent to the reputation manager
#[derive(Debug)]
enum ReputationCommand {
    /// Record a reputation change for a peer
    RecordChange {
        peer_id: PeerId,
        change_type: ReputationChange,
        response_tx: Option<tokio::sync::oneshot::Sender<NetworkResult<i32>>>,
    },
    
    /// Record response time for a peer's request
    RecordResponseTime {
        peer_id: PeerId,
        duration: Duration,
        response_tx: Option<tokio::sync::oneshot::Sender<NetworkResult<()>>>,
    },
    
    /// Check if a peer is banned
    IsBanned {
        peer_id: PeerId,
        response_tx: Option<tokio::sync::oneshot::Sender<bool>>,
    },
    
    /// Ban a peer
    BanPeer {
        peer_id: PeerId,
        response_tx: Option<tokio::sync::oneshot::Sender<NetworkResult<()>>>,
    },
    
    /// Unban a peer
    UnbanPeer {
        peer_id: PeerId,
        response_tx: Option<tokio::sync::oneshot::Sender<NetworkResult<()>>>,
    },
    
    /// Get the reputation for a peer
    GetReputation {
        peer_id: PeerId,
        response_tx: Option<tokio::sync::oneshot::Sender<Option<PeerReputation>>>,
    },
    
    /// Save current reputations to storage
    Save {
        response_tx: Option<tokio::sync::oneshot::Sender<NetworkResult<()>>>,
    },
    
    /// Stop the background task
    Stop {
        response_tx: Option<tokio::sync::oneshot::Sender<NetworkResult<()>>>,
    },
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
    /// Command senders
    command_senders: RwLock<Vec<mpsc::Sender<ReputationCommand>>>,
}

impl ReputationManager {
    /// Create a new reputation manager
    pub async fn new(
        config: ReputationConfig,
        storage: Option<Arc<dyn Storage>>,
        metrics: Option<NetworkMetrics>,
    ) -> NetworkResult<Self> {
        let reputations = Arc::new(RwLock::new(HashMap::new()));
        
        // Create a channel for commands
        let (command_tx, command_rx) = mpsc::channel(32);
        
        // Create the manager
        let manager = Self {
            reputations: Arc::clone(&reputations),
            config,
            command_tx,
            task_handle: RwLock::new(None),
            storage: storage.clone(),
            metrics: metrics.clone(),
            command_senders: RwLock::new(vec![]),
        };
        
        // Load existing reputations from storage if available
        if let Some(storage) = &storage {
            if let Ok(data) = storage.get(&config.storage_key).await {
                let rep_data: HashMap<String, PeerReputation> = serde_json::from_slice(&data)
                    .map_err(|_| NetworkError::DecodingError)?;
                
                let mut reputations_write = reputations.write().await;
                
                for (peer_id_str, reputation) in rep_data {
                    // Convert string peer ID to PeerId
                    if let Ok(peer_id) = PeerId::from_bytes(bs58::decode(&peer_id_str).into_vec().map_err(|_| NetworkError::DecodingError)?.as_slice()) {
                        reputations_write.insert(peer_id, reputation);
                    }
                }
                
                debug!("Loaded {} peer reputations from storage", reputations_write.len());
            }
        }
        
        // Start the background task
        let task = manager.start_decay_task();
        
        // Store the task handle
        {
            let mut handle = manager.task_handle.write();
            *handle = Some(task);
        }
        
        Ok(manager)
    }
    
    /// Start the reputation manager
    pub async fn start(&self) -> NetworkResult<()> {
        // Load stored reputation data if available
        if let Some(storage) = &self.storage {
            let storage_key = &self.config.storage_key;
            if !storage_key.is_empty() {
                if let Ok(data) = storage.get(storage_key).await {
                    if !data.is_empty() {
                        if let Ok(rep_data) = serde_json::from_slice::<HashMap<String, PeerReputation>>(&data) {
                            let mut reputations = self.reputations.write().await;
                            for (peer_id_str, reputation) in rep_data {
                                // Convert the peer ID string to a PeerId
                                let bytes = match bs58::decode(&peer_id_str).into_vec() {
                                    Ok(bytes) => bytes,
                                    Err(_) => {
                                        debug!("Failed to decode peer ID: {}", peer_id_str);
                                        continue;
                                    }
                                };
                                
                                if let Ok(peer_id) = PeerId::from_bytes(&bytes) {
                                    reputations.insert(peer_id, reputation);
                                    
                                    // Update metrics
                                    if let Some(metrics) = &self.metrics {
                                        metrics.update_reputation_score(&peer_id.to_string(), reputation.value);
                                        if reputation.banned {
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
        self.start_background_task().await?;
        
        Ok(())
    }
    
    /// Start the background task for reputation management
    pub async fn start_background_task(&self) -> NetworkResult<()> {
        let (command_tx, command_rx) = mpsc::channel(100);
        
        let mut senders = self.command_senders.write().await;
        senders.push(command_tx);
        
        let reputations = Arc::clone(&self.reputations);
        let metrics = self.metrics.clone();
        let storage = self.storage.clone();
        let config = self.config.clone();
        
        let task = tokio::spawn(async move {
            if let Err(e) = Self::run_background_task(
                command_rx,
                reputations,
                metrics,
                storage,
                config,
            ).await {
                error!("Reputation background task failed: {}", e);
            }
        });
        
        let mut handle = self.task_handle.write().await;
        *handle = Some(task);
        
        Ok(())
    }

    /// Actual implementation of the background task
    async fn run_background_task(
        mut command_rx: mpsc::Receiver<ReputationCommand>,
        reputations: Arc<RwLock<HashMap<PeerId, PeerReputation>>>,
        metrics: Option<NetworkMetrics>,
        storage: Option<Arc<dyn Storage>>,
        config: ReputationConfig,
    ) -> NetworkResult<()> {
        debug!("Starting reputation background task");
        
        // Create an interval for periodic decay
        let mut interval = tokio::time::interval(config.decay_interval);
        
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if let Err(e) = Self::handle_decay(&reputations, &metrics, &config).await {
                        error!("Error during reputation decay: {}", e);
                    }
                }
                
                Some(command) = command_rx.recv() => {
                    match command {
                        ReputationCommand::RecordChange { peer_id, change_type, response_tx } => {
                            let score = Self::handle_record_change(&reputations, &metrics, &peer_id, change_type).await;
                            if let Some(tx) = response_tx {
                                let _ = tx.send(Ok(score));
                            }
                        }
                        
                        ReputationCommand::RecordResponseTime { peer_id, duration, response_tx } => {
                            Self::handle_record_response_time(&reputations, &metrics, &peer_id, duration).await;
                            if let Some(tx) = response_tx {
                                let _ = tx.send(Ok(()));
                            }
                        }
                        
                        ReputationCommand::IsBanned { peer_id, response_tx } => {
                            let is_banned = Self::handle_is_banned(&reputations, &peer_id).await;
                            if let Some(tx) = response_tx {
                                let _ = tx.send(is_banned);
                            }
                        }
                        
                        ReputationCommand::BanPeer { peer_id, response_tx } => {
                            Self::handle_ban_peer(&reputations, &metrics, &peer_id).await;
                            if let Some(tx) = response_tx {
                                let _ = tx.send(Ok(()));
                            }
                        }
                        
                        ReputationCommand::UnbanPeer { peer_id, response_tx } => {
                            Self::handle_unban_peer(&reputations, &metrics, &peer_id).await;
                            if let Some(tx) = response_tx {
                                let _ = tx.send(Ok(()));
                            }
                        }
                        
                        ReputationCommand::GetReputation { peer_id, response_tx } => {
                            let rep = Self::handle_get_reputation(&reputations, &peer_id).await;
                            if let Some(tx) = response_tx {
                                let _ = tx.send(rep);
                            }
                        }
                        
                        ReputationCommand::Save { response_tx } => {
                            let result = Self::handle_save(&reputations, &storage, &config).await;
                            if let Some(tx) = response_tx {
                                let _ = tx.send(result);
                            }
                        }
                        
                        ReputationCommand::Stop { response_tx } => {
                            // Save reputations before stopping
                            let result = Self::handle_save(&reputations, &storage, &config).await;
                            if let Some(tx) = response_tx {
                                let _ = tx.send(result);
                            }
                            break;
                        }
                    }
                }
                
                else => {
                    debug!("All reputation command senders dropped, stopping background task");
                    // Try to save reputations before exiting
                    let _ = Self::handle_save(&reputations, &storage, &config).await;
                    break;
                }
            }
        }
        
        debug!("Reputation background task stopped");
        Ok(())
    }
    
    /// Handle a reputation change for a peer
    async fn handle_record_change(
        reputations: &RwLock<HashMap<PeerId, PeerReputation>>,
        metrics: &Option<NetworkMetrics>,
        peer_id: &PeerId,
        change: ReputationChange,
    ) -> i32 {
        let mut reputations = reputations.write().await;
        
        // Get or create the reputation entry
        let reputation = reputations
            .entry(peer_id.clone())
            .or_insert_with(PeerReputation::new);
        
        // Record the change
        let new_score = reputation.record_change(change);
        
        // Update metrics
        if let Some(m) = metrics {
            m.record_reputation_change(&peer_id.to_string(), change.value());
            m.update_reputation_score(&peer_id.to_string(), new_score);
            
            // Record specific metrics based on the change type
            match change {
                ReputationChange::ConnectionEstablished => {
                    // Connection metrics are handled elsewhere
                },
                ReputationChange::ConnectionLost => {
                    // Connection metrics are handled elsewhere
                },
                ReputationChange::MessageSuccess => {
                    m.record_positive_action(&peer_id.to_string(), "message_success");
                },
                ReputationChange::MessageFailure => {
                    m.record_negative_action(&peer_id.to_string(), "message_failure");
                },
                ReputationChange::InvalidMessage => {
                    m.record_negative_action(&peer_id.to_string(), "invalid_message");
                },
                ReputationChange::VerifiedMessage => {
                    m.record_positive_action(&peer_id.to_string(), "verified_message");
                },
                ReputationChange::DiscoveryHelp => {
                    m.record_positive_action(&peer_id.to_string(), "discovery_help");
                },
                ReputationChange::Misinformation => {
                    m.record_negative_action(&peer_id.to_string(), "misinformation");
                },
                ReputationChange::ExplicitBan => {
                    m.record_peer_banned(&peer_id.to_string());
                },
                ReputationChange::SlowResponse => {
                    m.record_negative_action(&peer_id.to_string(), "slow_response");
                },
                ReputationChange::FastResponse => {
                    m.record_positive_action(&peer_id.to_string(), "fast_response");
                },
                ReputationChange::AdminUnban => {
                    // Admin unban is handled elsewhere
                },
                ReputationChange::Manual(_) => {
                    // Manual changes don't need specific metrics
                },
            }
        }
        
        new_score
    }
    
    // Handle recording a response time
    async fn handle_record_response_time(
        reputations: &RwLock<HashMap<PeerId, PeerReputation>>,
        metrics: &Option<NetworkMetrics>,
        peer_id: &PeerId,
        duration: Duration,
    ) {
        let mut reputations = reputations.write().await;
        
        // Get or create the reputation entry
        let reputation = reputations
            .entry(peer_id.clone())
            .or_insert_with(PeerReputation::new);
        
        // Record the response time
        reputation.response_times.push(duration);
        
        // Keep only the most recent response times
        if reputation.response_times.len() > 10 { // Use a fixed value instead of config.max_response_times
            reputation.response_times.remove(0);
        }
        
        // Update the average response time
        let total_millis: u128 = reputation.response_times
            .iter()
            .map(|d| d.as_millis())
            .sum();
        
        let avg_millis = if reputation.response_times.is_empty() {
            0
        } else {
            total_millis / reputation.response_times.len() as u128
        };
        
        reputation.avg_response_time = Duration::from_millis(avg_millis as u64);
        
        // Update metrics
        if let Some(m) = metrics {
            m.record_operation_duration(&format!("peer_response_{}", peer_id), duration);
        }
    }
    
    // Handle checking if a peer is banned
    async fn handle_is_banned(
        reputations: &RwLock<HashMap<PeerId, PeerReputation>>,
        peer_id: &PeerId,
    ) -> bool {
        let reputations = reputations.read().await;
        
        match reputations.get(peer_id) {
            Some(rep) => rep.banned,
            None => false,
        }
    }
    
    // Handle banning a peer
    async fn handle_ban_peer(
        reputations: &RwLock<HashMap<PeerId, PeerReputation>>,
        metrics: &Option<NetworkMetrics>,
        peer_id: &PeerId,
    ) {
        let mut reputations = reputations.write().await;
        let reputation = reputations
            .entry(peer_id.clone())
            .or_insert_with(PeerReputation::new);
        
        reputation.banned = true;
        reputation.ban_time = Some(SystemTime::now());
        
        if let Some(ref m) = metrics {
            m.record_peer_banned(peer_id.to_string().as_str());
        }
    }
    
    // Handle unbanning a peer
    async fn handle_unban_peer(
        reputations: &RwLock<HashMap<PeerId, PeerReputation>>,
        metrics: &Option<NetworkMetrics>,
        peer_id: &PeerId,
    ) {
        let mut reputations = reputations.write().await;
        
        if let Some(reputation) = reputations.get_mut(peer_id) {
            reputation.banned = false;
            reputation.ban_time = None;
            
            if let Some(ref m) = metrics {
                m.record_peer_unbanned(peer_id.to_string().as_str());
            }
        }
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
            let storage_key = &config.storage_key;
            if !storage_key.is_empty() {
                // Convert PeerId to string for serialization
                let reputations_read = reputations.read().await;
                let mut serializable_reputations = HashMap::new();
                
                for (peer_id, reputation) in reputations_read.iter() {
                    serializable_reputations.insert(peer_id.to_string(), reputation.clone());
                }
                
                // Serialize and save
                let data = serde_json::to_vec(&serializable_reputations)
                    .map_err(|e| NetworkError::InternalError(format!("Failed to serialize reputations: {}", e)))?;
                
                storage.put(storage_key, &data).await
                    .map_err(|e| NetworkError::StorageError(e))?;
                
                debug!("Saved {} peer reputation records", serializable_reputations.len());
            }
        }
        
        Ok(())
    }
    
    // Handle reputation decay
    async fn handle_decay(
        reputations: &RwLock<HashMap<PeerId, PeerReputation>>,
        metrics: &Option<NetworkMetrics>,
        config: &ReputationConfig,
    ) -> NetworkResult<()> {
        let mut reputations = reputations.write().await;
        let decay_count = reputations.len();
        
        for (peer_id, reputation) in reputations.iter_mut() {
            let old_score = reputation.value;
            reputation.value = (old_score as f64 * config.decay_factor).ceil() as i32;
            
            // If score changed, update metrics
            if old_score != reputation.value && metrics.is_some() {
                if let Some(m) = metrics {
                    m.update_reputation_score(&peer_id.to_string(), reputation.value);
                }
            }
        }
        
        // Record decay processing in metrics
        if let Some(m) = metrics {
            m.record_reputation_decay(decay_count as u64);
        }
        
        Ok(())
    }
    
    /// Record a reputation change for a peer
    pub async fn record_change(&self, peer_id: PeerId, change_type: ReputationChange) -> NetworkResult<i32> {
        let (response_tx, response_rx) = tokio::sync::oneshot::channel();
        
        let cmd = ReputationCommand::RecordChange {
            peer_id,
            change_type,
            response_tx: Some(response_tx),
        };
        
        match self.send_command(cmd).await {
            Ok(_) => match response_rx.await {
                Ok(result) => result,
                Err(_) => Err(NetworkError::ChannelClosed("Reputation response channel closed".into())),
            },
            Err(e) => Err(e),
        }
    }
    
    /// Record response time for a peer
    pub async fn record_response_time(&self, peer_id: PeerId, duration: Duration) -> NetworkResult<()> {
        let (response_tx, response_rx) = tokio::sync::oneshot::channel();
        
        let cmd = ReputationCommand::RecordResponseTime {
            peer_id,
            duration,
            response_tx: Some(response_tx),
        };
        
        match self.send_command(cmd).await {
            Ok(_) => match response_rx.await {
                Ok(result) => result,
                Err(_) => Err(NetworkError::ChannelClosed("Reputation response channel closed".into())),
            },
            Err(e) => Err(e),
        }
    }
    
    /// Check if a peer is banned
    pub async fn is_banned(&self, peer_id: PeerId) -> bool {
        let (tx, rx) = tokio::sync::oneshot::channel();
        
        let cmd = ReputationCommand::IsBanned {
            peer_id,
            response_tx: Some(tx),
        };
        
        if let Err(e) = self.send_command(cmd).await {
            error!("Failed to send is_banned command: {}", e);
            return false;
        }
        
        match rx.await {
            Ok(is_banned) => is_banned,
            Err(e) => {
                error!("Failed to receive is_banned response: {}", e);
                false
            }
        }
    }
    
    /// Ban a peer manually
    pub async fn ban_peer(&self, peer_id: PeerId) -> NetworkResult<()> {
        let (response_tx, response_rx) = tokio::sync::oneshot::channel();
        
        let cmd = ReputationCommand::BanPeer {
            peer_id,
            response_tx: Some(response_tx),
        };
        
        match self.send_command(cmd).await {
            Ok(_) => match response_rx.await {
                Ok(result) => result,
                Err(_) => Err(NetworkError::ChannelClosed("Reputation response channel closed".into())),
            },
            Err(e) => Err(e),
        }
    }
    
    /// Unban a peer manually
    pub async fn unban_peer(&self, peer_id: PeerId) -> NetworkResult<()> {
        let (response_tx, response_rx) = tokio::sync::oneshot::channel();
        
        let cmd = ReputationCommand::UnbanPeer {
            peer_id,
            response_tx: Some(response_tx),
        };
        
        match self.send_command(cmd).await {
            Ok(_) => match response_rx.await {
                Ok(result) => result,
                Err(_) => Err(NetworkError::ChannelClosed("Reputation response channel closed".into())),
            },
            Err(e) => Err(e),
        }
    }
    
    /// Get the current reputation value for a peer
    pub async fn get_reputation(&self, peer_id: PeerId) -> NetworkResult<PeerReputation> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let cmd = ReputationCommand::GetReputation {
            peer_id,
            response_tx: Some(tx),
        };
        
        match self.send_command(cmd).await {
            Ok(_) => match rx.await {
                Ok(Some(rep)) => Ok(rep),
                Ok(None) => Ok(PeerReputation::default()), // Default reputation for unknown peers
                Err(e) => Err(NetworkError::Other(format!("Failed to get reputation: {}", e))),
            },
            Err(e) => Err(e),
        }
    }
    
    /// Save the current reputation state
    pub async fn save(&self) -> NetworkResult<()> {
        let (response_tx, response_rx) = tokio::sync::oneshot::channel();
        
        let cmd = ReputationCommand::Save {
            response_tx: Some(response_tx),
        };
        
        match self.send_command(cmd).await {
            Ok(_) => match response_rx.await {
                Ok(result) => result,
                Err(_) => Err(NetworkError::ChannelClosed("Reputation response channel closed".into())),
            },
            Err(e) => Err(e),
        }
    }
    
    /// Stop the reputation manager
    pub async fn stop(&self) -> NetworkResult<()> {
        let (response_tx, response_rx) = tokio::sync::oneshot::channel();
        
        let cmd = ReputationCommand::Stop {
            response_tx: Some(response_tx),
        };
        
        match self.send_command(cmd).await {
            Ok(_) => match response_rx.await {
                Ok(result) => result,
                Err(_) => Err(NetworkError::ChannelClosed("Reputation response channel closed".into())),
            },
            Err(e) => Err(e),
        }
    }
    
    /// Get an immutable reference to the reputations
    pub async fn reputations(&self) -> tokio::sync::RwLockReadGuard<'_, HashMap<PeerId, PeerReputation>> {
        self.reputations.read().await
    }
    
    /// Start the decay task
    pub async fn start_decay_task(&self) -> JoinHandle<()> {
        let reputations = Arc::clone(&self.reputations);
        let metrics = self.metrics.clone();
        let storage = self.storage.clone();
        let config = self.config.clone();
        
        // Create a channel for reputation commands
        let (command_tx, command_rx) = mpsc::channel(32);
        
        // Store the sender in the list of command senders
        {
            let mut senders = self.command_senders.write().await;
            senders.push(command_tx);
        }
        
        // Spawn the background task
        tokio::spawn(async move {
            if let Err(e) = Self::run_background_task(command_rx, reputations, metrics, storage, config).await {
                error!("Reputation decay task failed: {}", e);
            }
        })
    }

    /// Send a command to the background task
    async fn send_command(&self, cmd: ReputationCommand) -> NetworkResult<()> {
        // Try to send the command to the background task
        if let Err(e) = self.command_tx.send(cmd).await {
            return Err(NetworkError::Other(format!("Failed to send reputation command: {}", e)));
        }
        
        Ok(())
    }

    /// Add a new method to get just the reputation value
    pub async fn get_reputation_value(&self, peer_id: PeerId) -> NetworkResult<i32> {
        match self.get_reputation(peer_id).await {
            Ok(rep) => Ok(rep.value),
            Err(e) => Err(e),
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
        assert_eq!(rep1.value, 1); // FastResponse gives +1
        
        // Slow response
        manager.record_response_time(peer_id, Duration::from_millis(300)).await.unwrap();
        let rep2 = manager.get_reputation(peer_id).await.unwrap();
        // Weighted average: (30*9 + 300)/10 = 57
        assert_eq!(rep2.response_times.len(), 2);
        assert_eq!(rep2.value, -1); // SlowResponse gives -2 after +1
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