//! Network messaging module for ICN
//!
//! This module handles message encoding, decoding, and processing 
//! for communication between nodes in the InterCooperative Network.

use std::collections::{HashMap, BinaryHeap, VecDeque};
use std::sync::Arc;
use std::cmp::Ordering;
use std::time::{Duration, Instant, SystemTime};

use async_trait::async_trait;
use tokio::sync::{mpsc, RwLock};
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn, trace};
use serde::{Serialize, Deserialize};

use crate::{NetworkError, NetworkResult, NetworkMessage, MessageHandler, PeerInfo};
use crate::reputation::{ReputationManager, ReputationChange};
use crate::metrics::NetworkMetrics;
use libp2p::PeerId;
use icn_core::storage::Storage;

use crate::NetworkService;

/// Maximum number of messages to process in a single batch
const MAX_MESSAGES_PER_BATCH: usize = 10;

/// Message type identifier
pub type MessageType = String;

/// Message handler function type
pub type MessageHandlerFn = Arc<dyn MessageHandler>;

/// Quality of Service levels for message prioritization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum QosLevel {
    /// Critical system messages (e.g., consensus)
    Critical,
    /// High priority messages (e.g., governance)
    High,
    /// Normal priority messages (e.g., regular transactions)
    Normal,
    /// Low priority messages (e.g., peer discovery)
    Low,
    /// Background tasks (e.g., state sync)
    Background,
}

impl QosLevel {
    /// Get the numeric priority value (higher is more important)
    pub fn priority_value(&self) -> u8 {
        match self {
            QosLevel::Critical => 255,
            QosLevel::High => 192,
            QosLevel::Normal => 128,
            QosLevel::Low => 64,
            QosLevel::Background => 0,
        }
    }

    /// Get the maximum queue size for this QoS level
    pub fn max_queue_size(&self) -> usize {
        match self {
            QosLevel::Critical => 1000,    // Critical messages need guaranteed delivery
            QosLevel::High => 5000,        // High priority but still limited
            QosLevel::Normal => 10000,     // Regular operation queue size
            QosLevel::Low => 20000,        // Can handle more low priority messages
            QosLevel::Background => 50000,  // Large queue for background tasks
        }
    }

    /// Get the timeout for this QoS level
    pub fn timeout(&self) -> Duration {
        match self {
            QosLevel::Critical => Duration::from_secs(5),
            QosLevel::High => Duration::from_secs(10),
            QosLevel::Normal => Duration::from_secs(30),
            QosLevel::Low => Duration::from_secs(60),
            QosLevel::Background => Duration::from_secs(300),
        }
    }
}

/// Message queue entry with metadata
#[derive(Debug)]
struct QueueEntry {
    /// The actual message
    message: NetworkMessage,
    /// When the message was queued
    queued_at: Instant,
    /// Number of delivery attempts
    attempts: u32,
    /// QoS level for this message
    qos_level: QosLevel,
}

/// A message queued for processing
#[derive(Debug, Clone)]
pub struct QueuedMessage {
    /// The message to be sent
    pub message: NetworkMessage,
    /// The sender peer id
    pub sender: String,
    /// QoS level for this message
    pub qos_level: QosLevel,
    /// When the message was queued
    pub queued_at: Instant,
    /// Number of delivery attempts
    pub attempts: u32,
}

/// Configuration for message prioritization
#[derive(Debug, Clone)]
pub struct PriorityConfig {
    /// Maximum queue size per peer
    pub max_queue_size: usize,
    /// Maximum number of delivery attempts
    pub max_attempts: u32,
    /// Enable dynamic QoS adjustment
    pub enable_dynamic_qos: bool,
    /// Base timeout for message delivery
    pub base_timeout: Duration,
}

impl Default for PriorityConfig {
    fn default() -> Self {
        Self {
            max_queue_size: 100000,
            max_attempts: 3,
            enable_dynamic_qos: true,
            base_timeout: Duration::from_secs(30),
        }
    }
}

/// Message envelope containing the message and metadata
#[derive(Debug, Clone)]
pub struct MessageEnvelope {
    /// The message
    pub message: NetworkMessage,
    /// The peer that sent the message
    pub peer: PeerInfo,
    /// When the message was received
    pub received_at: Instant,
    /// Priority of the message (higher = more important)
    pub priority: i32,
}

impl PartialEq for MessageEnvelope {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority
    }
}

impl Eq for MessageEnvelope {}

impl PartialOrd for MessageEnvelope {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for MessageEnvelope {
    fn cmp(&self, other: &Self) -> Ordering {
        // Higher priority comes first (max-heap)
        self.priority.cmp(&other.priority)
    }
}

/// Priority calculation mode for messages
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PriorityMode {
    /// First in, first out (no prioritization)
    Fifo,
    /// Priority based on peer reputation (higher rep = higher priority)
    ReputationBased,
    /// Priority based on message type and peer reputation
    TypeAndReputation,
    /// Priority based on custom criteria
    Custom,
}

impl Default for PriorityMode {
    fn default() -> Self {
        Self::Fifo
    }
}

/// Message processor with prioritization
pub struct MessageProcessor {
    /// Configuration
    config: PriorityConfig,
    /// Message queues per peer and QoS level
    queues: Arc<RwLock<HashMap<String, HashMap<QosLevel, VecDeque<QueueEntry>>>>>,
    /// Network service for sending messages
    pub network: Arc<dyn NetworkService>,
    /// Storage for persisting messages
    pub storage: Option<Arc<dyn Storage>>,
    /// Message handlers
    pub handlers: Arc<RwLock<HashMap<MessageType, Vec<MessageHandlerFn>>>>,
    /// Task handle for the background processor
    pub task_handle: RwLock<Option<JoinHandle<()>>>,
    /// Whether the processor is running
    pub running: RwLock<bool>,
    /// Reputation manager
    pub reputation: Option<Arc<ReputationManager>>,
    /// Network metrics
    pub metrics: Option<NetworkMetrics>,
    /// Command sender
    pub command_tx: mpsc::Sender<ProcessorCommand>,
}

impl Clone for MessageProcessor {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            queues: self.queues.clone(),
            network: self.network.clone(),
            storage: self.storage.clone(),
            handlers: self.handlers.clone(),
            task_handle: RwLock::new(None), // Don't clone the task handle
            running: RwLock::new(*self.running.blocking_read()),
            reputation: self.reputation.clone(),
            metrics: self.metrics.clone(),
            command_tx: self.command_tx.clone(),
        }
    }
}

/// Command for controlling the message processor
enum ProcessorCommand {
    /// Process a message
    ProcessMessage(MessageEnvelope),
    /// Stop the processor
    Stop(mpsc::Sender<NetworkResult<()>>),
}

impl MessageProcessor {
    /// Create a new message processor
    pub fn new(
        handlers: Arc<RwLock<HashMap<String, Vec<Arc<dyn MessageHandler>>>>>,
        config: PriorityConfig,
        reputation: Option<Arc<ReputationManager>>,
        metrics: Option<NetworkMetrics>,
        network: Arc<dyn NetworkService>,
        storage: Option<Arc<dyn Storage>>,
    ) -> Self {
        let (command_tx, command_rx) = mpsc::channel(config.max_queue_size);
        
        let processor = Self {
            handlers,
            queues: Arc::new(RwLock::new(HashMap::new())),
            config,
            reputation,
            metrics,
            command_tx,
            task_handle: RwLock::new(None),
            running: RwLock::new(false),
            network,
            storage,
        };
        
        // Start the background processing task
        processor.start_background_task(command_rx);
        
        processor
    }
    
    /// Start the background processing task
    fn start_background_task(&self, mut command_rx: mpsc::Receiver<ProcessorCommand>) {
        let handlers = Arc::clone(&self.handlers);
        let queues = Arc::clone(&self.queues);
        let config = self.config.clone();
        let reputation = self.reputation.clone();
        let metrics = self.metrics.clone();
        
        let running = Arc::new(tokio::sync::RwLock::new(true));
        let running_clone = Arc::clone(&running);
        
        // Create a processor clone to use in the spawned task
        let processor_clone = self.clone();
        
        let task = tokio::spawn(async move {
            *running_clone.write().await = true;
            
            while let Some(command) = command_rx.recv().await {
                match command {
                    ProcessorCommand::ProcessMessage(envelope) => {
                        // Call queue_message directly with the envelope data
                        processor_clone.queue_message(
                            &envelope.peer.peer_id, 
                            envelope.message.clone(), 
                            QosLevel::Normal // Use a default QoS level for now
                        ).await;
                        
                        // Record queue size in metrics
                        if let Some(m) = &metrics {
                            let size = processor_clone.queue_size().await;
                            m.record_queue_size(size);
                        }
                    },
                    ProcessorCommand::Stop(response_tx) => {
                        *running_clone.write().await = false;
                        let _ = response_tx.send(Ok(())).await;
                        break;
                    }
                }
                
                // Process messages from the queue while there are any
                processor_clone.process_queue(
                    &handlers,
                    &queues,
                    &reputation,
                    &metrics,
                ).await;
            }
            
            debug!("Message processor background task stopped");
        });
        
        // Create new clones for the periodic task
        let handlers_periodic = Arc::clone(&self.handlers);
        let queues_periodic = Arc::clone(&self.queues);
        let reputation_periodic = self.reputation.clone();
        let metrics_periodic = self.metrics.clone();
        let running_periodic = Arc::clone(&running);
        let processor = self.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(50));
            
            while *running_periodic.read().await {
                interval.tick().await;
                
                // Process messages from the queue periodically
                processor.process_queue(
                    &handlers_periodic,
                    &queues_periodic,
                    &reputation_periodic,
                    &metrics_periodic,
                ).await;
            }
            
            debug!("Periodic message processor task stopped");
        });
        
        let mut handle = futures::executor::block_on(self.task_handle.write());
        *handle = Some(task);
    }
    
    /// Process messages from the queue
    async fn process_queue(
        &self,
        handlers: &Arc<RwLock<HashMap<String, Vec<Arc<dyn MessageHandler>>>>>,
        queues: &Arc<RwLock<HashMap<String, HashMap<QosLevel, VecDeque<QueueEntry>>>>>,
        reputation: &Option<Arc<ReputationManager>>,
        metrics: &Option<NetworkMetrics>,
    ) {
        let mut processed = 0;
        
        // Process up to MAX_MESSAGES_PER_BATCH messages
        for _ in 0..MAX_MESSAGES_PER_BATCH {
            // Get the highest priority message from the queue
            let envelope = {
                let mut queues_write = queues.write().await;
                if queues_write.is_empty() {
                    break;
                }
                let envelope = queues.read().await.iter()
                    .flat_map(move |(peer_id, peer_queues)| {
                        peer_queues.iter().flat_map(move |(qos_level, queue)| {
                            queue.iter().map(move |entry| {
                                (peer_id.clone(), qos_level.clone(), entry.message.clone())
                            })
                        })
                    }).max_by(|(_, qos_level1, _), (_, qos_level2, _)| {
                        qos_level1.cmp(&qos_level2)
                    }).map(|(peer_id, qos_level, message)| {
                        (peer_id, qos_level, message)
                    });
                
                envelope
            };
            
            if let Some((peer_id, qos_level, message)) = envelope {
                // Extract message type and peer
                let message_type = message.message_type();
                let sender = &peer_id;
                
                // Start timing the message processing
                let process_start = Instant::now();
                
                // Find handlers for this message type
                let handlers = handlers.read().await
                    .get(&message_type)
                    .cloned();
                
                if let Some(type_handlers) = handlers {
                    // We need to deserialize the message data based on the message type
                    // For now, we'll just log that we're processing the message
                    debug!("Processing message of type {} from {:?}", message_type, sender);
                    
                    // In a real implementation, we would:
                    // 1. Deserialize the message data into the appropriate type
                    // 2. Create a NetworkMessage from it
                    // 3. Create a PeerInfo from the sender
                    // 4. Call the handler with the NetworkMessage and PeerInfo
                    
                    // For now, we'll just record the processing time
                    let process_duration = process_start.elapsed();
                    debug!("Processed message in {:?}", process_duration);
                    
                    // Record success in reputation system if available
                    if let Some(rep) = reputation {
                        if let Ok(peer_id) = PeerId::from_bytes(sender.as_bytes()) {
                            if let Err(e) = rep.record_change(peer_id, ReputationChange::MessageSuccess).await {
                                error!("Failed to update reputation: {}", e);
                            }
                        }
                    }
                    
                    processed += 1;
                } else {
                    debug!("No handlers registered for message type: {}", message_type);
                }
            } else {
                break;
            }
        }
        
        // Update metrics with current queue size
        if let Some(m) = metrics {
            let size = queues.read().await.iter().flat_map(|(peer_id, peer_queues)| {
                peer_queues.iter().map(|(qos_level, queue)| {
                    queue.len()
                })
            }).sum();
            m.record_queue_size(size);
        }
        
        if processed > 0 {
            trace!("Processed {} messages from queue", processed);
        }
    }
    
    /// Calculate the priority of a message based on configuration and peer reputation
    async fn calculate_priority(
        &self,
        message: &NetworkMessage,
        peer: &PeerInfo,
    ) -> i32 {
        // Base priority is 0
        let mut priority = 0;
        
        // If reputation manager is available, boost priority based on peer reputation
        if let Some(rep) = &self.reputation {
            let context = crate::reputation::ReputationContext::Networking;
            
            // Convert string to PeerId
            match PeerId::from_bytes(peer.peer_id.as_bytes()) {
                Ok(peer_id) => {
                    // Adjust priority based on reputation score
                    let reputation_score = rep.get_reputation(&peer_id, &context);
                    
                    if reputation_score > 50 {
                        priority += 2; // High reputation peers get higher priority
                    } else if reputation_score < 0 {
                        priority -= 1; // Low reputation peers get lower priority
                    }
                },
                Err(_) => {
                    // If we can't parse peer_id, default to no reputation adjustment
                    debug!("Failed to parse peer_id: {}", peer.peer_id);
                }
            }
        }
        
        // Add priority based on message type
        match message.message_type().as_str() {
            // System messages get highest priority
            "consensus" | "governance" | "voting" => priority += 3,
            // Regular messages get normal priority
            "transaction" | "content" | "data" => priority += 1,
            // Discovery and background tasks get lower priority
            "discovery" | "ping" | "status" => priority += 0,
            // Default priority for unknown types
            _ => {},
        }
        
        priority
    }
    
    /// Process a message with appropriate priority
    pub async fn process_message(
        &self,
        message: NetworkMessage,
        peer: PeerInfo,
    ) -> NetworkResult<()> {
        // Check if the processor is running
        if !*self.running.read().await {
            return Err(NetworkError::ServiceStopped);
        }
        
        // Calculate the priority
        let priority = self.calculate_priority(&message, &peer).await;
        
        // Create the message envelope
        let envelope = MessageEnvelope {
            message,
            peer: peer.clone(),
            received_at: Instant::now(),
            priority,
        };
        
        // Check queue size before adding
        let queue_size = self.queues.read().await.get(&peer.peer_id).unwrap().len();
        if queue_size >= self.config.max_queue_size {
            // Apply backpressure by dropping lowest priority messages if needed
            if priority < 0 {
                // For negative priority messages, allow dropping
                let _ = self.apply_backpressure(&mut self.queues.write().await.get_mut(&peer.peer_id).unwrap()).await;
                return Err(NetworkError::InvalidPriority);
            }
            
            // We're at capacity but this message isn't low priority, so apply backpressure
            warn!("Message queue full, applying backpressure ({} messages waiting)", queue_size);
            
            // Record backpressure in metrics
            if let Some(m) = &self.metrics {
                m.record_backpressure();
            }
        }
        
        // Send the message to the background task for processing
        if let Err(e) = self.command_tx.send(ProcessorCommand::ProcessMessage(envelope)).await {
            error!("Failed to send message to processor: {}", e);
            return Err(NetworkError::ServiceError(format!("Failed to send message to processor: {}", e)));
        }
        
        Ok(())
    }
    
    /// Stop the message processor
    pub async fn stop(&self) -> NetworkResult<()> {
        let (tx, mut rx) = mpsc::channel(1);
        
        if let Err(e) = self.command_tx.send(ProcessorCommand::Stop(tx)).await {
            return Err(NetworkError::ServiceError(format!("Failed to send stop command: {}", e)));
        }
        
        match rx.recv().await {
            Some(result) => result,
            None => Err(NetworkError::ServiceError("Channel closed before receiving response".to_string())),
        }
    }
    
    /// Get the current queue size
    pub async fn queue_size(&self) -> usize {
        self.queues.read().await.iter().flat_map(|(peer_id, peer_queues)| {
            peer_queues.iter().map(|(qos_level, queue)| {
                queue.len()
            })
        }).sum()
    }
    
    /// Get queue statistics
    pub async fn queue_stats(&self) -> (usize, Option<i32>, Option<i32>) {
        let size = self.queue_size().await;
        
        if size == 0 {
            return (0, None, None);
        }
        
        let queues = self.queues.read().await;
        
        // Find the highest and lowest priority messages
        let mut highest_priority: Option<i32> = None;
        let mut lowest_priority: Option<i32> = None;
        
        for (_, peer_queues) in queues.iter() {
            for (qos_level, queue) in peer_queues.iter() {
                if !queue.is_empty() {
                    let priority = qos_level.priority_value() as i32;
                    
                    // Update highest priority
                    if let Some(current_highest) = highest_priority {
                        if priority > current_highest {
                            highest_priority = Some(priority);
                        }
                    } else {
                        highest_priority = Some(priority);
                    }
                    
                    // Update lowest priority
                    if let Some(current_lowest) = lowest_priority {
                        if priority < current_lowest {
                            lowest_priority = Some(priority);
                        }
                    } else {
                        lowest_priority = Some(priority);
                    }
                }
            }
        }
        
        (size, highest_priority, lowest_priority)
    }
    
    /// Add a message directly to the queue
    pub async fn push_back(&self, message: QueuedMessage) {
        let mut queues = self.queues.write().await;
        let peer_queues = queues
            .entry(message.sender.clone())
            .or_insert_with(HashMap::new);
        
        let queue = peer_queues
            .entry(message.qos_level)
            .or_insert_with(VecDeque::new);
            
        // Convert QueuedMessage to QueueEntry
        let entry = QueueEntry {
            message: message.message,
            queued_at: message.queued_at,
            attempts: message.attempts,
            qos_level: message.qos_level,
        };
        
        queue.push_back(entry);
    }

    /// Queue a message for delivery
    pub async fn queue_message(
        &self,
        peer_id: &str,
        message: NetworkMessage,
        qos_level: QosLevel,
    ) -> NetworkResult<()> {
        let mut queues = self.queues.write().await;
        let peer_queues = queues
            .entry(peer_id.to_string())
            .or_insert_with(HashMap::new);
            
        // Check queue size limits before adding
        let queue_size = peer_queues.values().map(|q| q.len()).sum::<usize>();
        if queue_size >= self.config.max_queue_size {
            // Try to free up space by dropping lowest priority messages
            let mut did_free_space = false;
            
            // Try dropping from lowest priority queues first
            for level in [QosLevel::Background, QosLevel::Low, QosLevel::Normal] {
                if let Some(queue) = peer_queues.get_mut(&level) {
                    if !queue.is_empty() {
                        queue.pop_front();
                        did_free_space = true;
                        break;
                    }
                }
            }
            
            // If we couldn't free up space and this is not a high priority message,
            // return an error
            if !did_free_space && qos_level < QosLevel::High {
                return Err(NetworkError::QueueFull);
            }
        }
        
        // Now add to the queue
        let queue = peer_queues
            .entry(qos_level)
            .or_insert_with(VecDeque::new);
        
        // Create queue entry
        let entry = QueueEntry {
            message,
            queued_at: Instant::now(),
            attempts: 0,
            qos_level,
        };
        
        // Add to queue
        queue.push_back(entry);
        
        // Update metrics
        if let Some(metrics) = &self.metrics {
            metrics.record_queued_message(peer_id, qos_level.priority_value());
        }
        
        Ok(())
    }

    /// Get the next message to process
    pub async fn next_message(&self, peer_id: &str) -> Option<NetworkMessage> {
        let mut queues = self.queues.write().await;
        
        if let Some(peer_queues) = queues.get_mut(peer_id) {
            // Try each QoS level in priority order
            for qos_level in [
                QosLevel::Critical,
                QosLevel::High,
                QosLevel::Normal,
                QosLevel::Low,
                QosLevel::Background,
            ] {
                if let Some(queue) = peer_queues.get_mut(&qos_level) {
                    // Get next message that hasn't timed out
                    while let Some(entry) = queue.front() {
                        if entry.queued_at.elapsed() > qos_level.timeout() {
                            // Message timed out, remove it
                            queue.pop_front();
                            if let Some(metrics) = &self.metrics {
                                metrics.record_message_timeout(peer_id, qos_level.priority_value());
                            }
                            continue;
                        }
                        
                        // Valid message found
                        if let Some(entry) = queue.pop_front() {
                            // Record stats
                            if let Some(metrics) = &self.metrics {
                                metrics.record_message_processed(
                                    peer_id,
                                    entry.message.message_type().as_str(),
                                    qos_level.priority_value(),
                                    entry.queued_at.elapsed()
                                );
                            }
                            return Some(entry.message);
                        }
                    }
                }
            }
        }
        
        None
    }

    /// Apply backpressure by dropping low priority messages
    async fn apply_backpressure(
        &self,
        peer_queues: &mut HashMap<QosLevel, VecDeque<QueueEntry>>,
    ) -> bool {
        // First check if we have any Background messages we can drop
        if let Some(background_queue) = peer_queues.get_mut(&QosLevel::Background) {
            if !background_queue.is_empty() {
                background_queue.pop_front();
                return true;
            }
        }
        
        // Then try Low priority messages
        if let Some(low_queue) = peer_queues.get_mut(&QosLevel::Low) {
            if !low_queue.is_empty() {
                low_queue.pop_front();
                return true;
            }
        }
        
        // If dynamic QoS is enabled, we can also drop Normal priority messages
        // but only if we're running out of space
        if self.config.enable_dynamic_qos {
            let total_messages: usize = peer_queues.values().map(|q| q.len()).sum();
            if total_messages > self.config.max_queue_size / 2 {
                if let Some(normal_queue) = peer_queues.get_mut(&QosLevel::Normal) {
                    if !normal_queue.is_empty() {
                        normal_queue.pop_front();
                        return true;
                    }
                }
            }
        }
        
        // We couldn't drop any messages, queue is full
        false
    }

    /// Get queue statistics for a peer
    pub async fn get_queue_stats(&self, peer_id: &str) -> (usize, usize, Duration) {
        let queues = self.queues.read().await;
        
        if let Some(peer_queues) = queues.get(peer_id) {
            let mut total_messages = 0;
            let mut max_queue_size = 0;
            let mut oldest_message = Duration::from_secs(0);
            
            for (qos_level, queue) in peer_queues {
                total_messages += queue.len();
                max_queue_size = max_queue_size.max(qos_level.max_queue_size());
                
                if let Some(entry) = queue.front() {
                    oldest_message = oldest_message.max(entry.queued_at.elapsed());
                }
            }
            
            (total_messages, max_queue_size, oldest_message)
        } else {
            (0, 0, Duration::from_secs(0))
        }
    }

    /// Clean up expired messages and update metrics
    pub async fn cleanup(&self) {
        let mut queues = self.queues.write().await;
        
        for (peer_id, peer_queues) in queues.iter_mut() {
            for (qos_level, queue) in peer_queues.iter_mut() {
                // Remove expired messages
                while let Some(entry) = queue.front() {
                    if entry.queued_at.elapsed() > qos_level.timeout() {
                        queue.pop_front();
                        if let Some(metrics) = &self.metrics {
                            metrics.record_message_timeout(peer_id, qos_level.priority_value());
                        }
                    } else {
                        break;
                    }
                }
            }
        }
    }

    pub async fn queue_for_send(
        &self,
        peer: PeerInfo,
        message: NetworkMessage,
        priority: Option<i32>,
    ) -> NetworkResult<()> {
        let peer_id = peer.peer_id.clone();
        
        // Calculate message priority
        let actual_priority = if let Some(p) = priority {
            p
        } else {
            self.calculate_priority(&message, &peer).await
        };
        
        // Create message envelope
        let envelope = MessageEnvelope {
            peer: peer.clone(),
            message,
            priority: actual_priority,
            received_at: Instant::now(),
        };
        
        // Check queue limits
        let queue_size = self.queues.read().await.get(&peer_id).map_or(0, |q| q.len());
        if queue_size >= self.config.max_queue_size {
            // Apply backpressure by dropping lowest priority messages if needed
            if priority.unwrap_or(0) < 0 {
                // For negative priority messages, allow dropping
                let _ = self.apply_backpressure(&mut self.queues.write().await.get_mut(&peer_id).unwrap()).await;
                return Err(NetworkError::InvalidPriority);
            }
        }
        
        // Send to command channel
        if let Err(_) = self.command_tx.send(ProcessorCommand::ProcessMessage(envelope)).await {
            return Err(NetworkError::ChannelClosed("Message processor channel closed".to_string()));
        }
        
        Ok(())
    }
}

/// Default message handler implementation that uses a closure
pub struct DefaultMessageHandler {
    /// Handler ID
    id: usize,
    /// Handler name
    name: String,
    /// Handler function
    handler: Box<dyn Fn(&NetworkMessage, &PeerInfo) -> NetworkResult<()> + Send + Sync>,
}

impl DefaultMessageHandler {
    /// Create a new default message handler
    pub fn new<F>(id: usize, name: String, handler: F) -> Self
    where
        F: Fn(&NetworkMessage, &PeerInfo) -> NetworkResult<()> + Send + Sync + 'static,
    {
        Self {
            id,
            name,
            handler: Box::new(handler),
        }
    }
}

#[async_trait]
impl MessageHandler for DefaultMessageHandler {
    fn id(&self) -> usize {
        self.id
    }
    
    fn name(&self) -> &str {
        &self.name
    }
    
    async fn handle_message(&self, message: &NetworkMessage, peer: &PeerInfo) -> NetworkResult<()> {
        (self.handler)(message, peer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{TransactionAnnouncement, CustomMessage};
    use std::sync::atomic::{AtomicBool, Ordering};
    
    #[tokio::test]
    async fn test_message_encoding_decoding() {
        let processor = MessageProcessor::new();
        
        // Create a test message
        let tx_announce = TransactionAnnouncement {
            transaction_id: "tx123".to_string(),
            transaction_type: "transfer".to_string(),
            timestamp: 12345,
            sender: "alice".to_string(),
            data_hash: "abcdef123456".to_string(),
        };
        
        let message = NetworkMessage::TransactionAnnouncement(tx_announce);
        
        // Encode the message
        let envelope = processor.encode_message(
            "peer123".to_string(),
            &message,
            None
        ).await.unwrap();
        
        // Check the envelope
        assert_eq!(envelope.sender, "peer123");
        assert_eq!(envelope.message_type, "ledger.transaction");
        assert!(envelope.payload.len() > 0);
        assert!(envelope.signature.is_none());
        
        // Decode the message
        let decoded = processor.decode_message(&envelope).await.unwrap();
        
        // Check the decoded message
        match decoded {
            NetworkMessage::TransactionAnnouncement(tx) => {
                assert_eq!(tx.transaction_id, "tx123");
                assert_eq!(tx.transaction_type, "transfer");
                assert_eq!(tx.timestamp, 12345);
                assert_eq!(tx.sender, "alice");
                assert_eq!(tx.data_hash, "abcdef123456");
            },
            _ => panic!("Unexpected message type"),
        }
    }
    
    #[tokio::test]
    async fn test_message_handlers() {
        let processor = MessageProcessor::new();
        
        // Create a flag to check if the handler was called
        let handler1_called = Arc::new(AtomicBool::new(false));
        let handler1_called_clone = handler1_called.clone();
        
        // Create a handler
        let handler1 = Arc::new(DefaultMessageHandler::new(
            1,
            "Test Handler".to_string(),
            move |message, peer| {
                handler1_called_clone.store(true, Ordering::SeqCst);
                
                // Check the message and peer
                match message {
                    NetworkMessage::CustomMessage(custom) => {
                        assert_eq!(custom.message_type, "test");
                        assert_eq!(custom.data["key"], "value");
                    },
                    _ => panic!("Unexpected message type"),
                }
                
                assert_eq!(peer.peer_id.to_string(), "peer456");
                
                Ok(())
            }
        ));
        
        // Register the handler
        processor.register_handler("test", handler1).await;
        
        // Create a test message
        let custom_data = {
            let mut map = serde_json::Map::new();
            map.insert("key".to_string(), serde_json::Value::String("value".to_string()));
            map
        };
        
        let custom = CustomMessage {
            message_type: "test".to_string(),
            data: custom_data,
        };
        
        // Create a peer
        let peer = PeerInfo {
            peer_id: "peer456".parse().unwrap(),
            addresses: vec![],
            protocols: vec![],
            connected: true,
            last_seen: 0,
        };
        
        // Create an envelope
        let envelope = MessageEnvelope::new(
            "peer123".to_string(),
            "test".to_string(),
            serde_json::to_vec(&custom).unwrap(),
            12345,
            None
        );
        
        // Process the message
        processor.process_message(&envelope, &peer).await.unwrap();
        
        // Check if the handler was called
        assert!(handler1_called.load(Ordering::SeqCst));
        
        // Unregister the handler
        let result = processor.unregister_handler("test", 1).await;
        assert!(result);
        
        // Reset the flag
        handler1_called.store(false, Ordering::SeqCst);
        
        // Process the message again
        processor.process_message(&envelope, &peer).await.unwrap();
        
        // Check that the handler was not called this time
        assert!(!handler1_called.load(Ordering::SeqCst));
    }
} 