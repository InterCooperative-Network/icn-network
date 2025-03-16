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

/// Configuration for the message processor
#[derive(Debug, Clone)]
pub struct MessageProcessorConfig {
    /// Maximum number of messages to process in a batch
    pub batch_size: usize,
    /// Interval between processing batches
    pub process_interval: Duration,
    /// Whether to persist messages
    pub persist_messages: bool,
    /// Storage key prefix for messages
    pub storage_key_prefix: String,
}

impl Default for MessageProcessorConfig {
    fn default() -> Self {
        Self {
            batch_size: 100,
            process_interval: Duration::from_millis(100),
            persist_messages: false,
            storage_key_prefix: "messages:".to_string(),
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

/// Configuration for priority-based message processing
#[derive(Debug, Clone)]
pub struct PriorityConfig {
    /// Prioritization mode
    pub mode: PriorityMode,
    /// Base priority for each message type
    pub type_priorities: HashMap<String, i32>,
    /// Minimum reputation score needed for high priority
    pub high_priority_reputation: i32,
    /// Maximum queue size before applying backpressure
    pub max_queue_size: usize,
    /// Whether to drop low-priority messages when queue is full
    pub drop_low_priority_when_full: bool,
}

impl Default for PriorityConfig {
    fn default() -> Self {
        let mut type_priorities = HashMap::new();
        // Set default priorities for message types
        type_priorities.insert("identity.announcement".to_string(), 80);
        type_priorities.insert("ledger.transaction".to_string(), 60);
        type_priorities.insert("ledger.state".to_string(), 70);
        type_priorities.insert("governance.proposal".to_string(), 50);
        type_priorities.insert("governance.vote".to_string(), 40);
        
        Self {
            mode: PriorityMode::ReputationBased,
            type_priorities,
            high_priority_reputation: 20,
            max_queue_size: 1000,
            drop_low_priority_when_full: true,
        }
    }
}

/// A message queued for processing
#[derive(Debug, Clone)]
pub struct QueuedMessage {
    /// Message type
    pub message_type: MessageType,
    /// Message data
    pub data: Vec<u8>,
    /// Sender peer ID
    pub sender: Option<String>,
    /// When the message was received
    pub received_at: Instant,
    /// Priority of the message (higher is more important)
    pub priority: i32,
}

/// Message processor for handling incoming and outgoing messages
pub struct MessageProcessor {
    /// Configuration
    pub config: PriorityConfig,
    /// Network service for sending messages
    pub network: Arc<dyn NetworkService>,
    /// Storage for persisting messages
    pub storage: Option<Arc<dyn Storage>>,
    /// Message handlers
    pub handlers: Arc<RwLock<HashMap<MessageType, Vec<MessageHandlerFn>>>>,
    /// Queue of messages to process
    pub queue: Arc<RwLock<VecDeque<QueuedMessage>>>,
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
            network: self.network.clone(),
            storage: self.storage.clone(),
            handlers: self.handlers.clone(),
            queue: self.queue.clone(),
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
            queue: Arc::new(RwLock::new(VecDeque::new())),
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
        let queue = Arc::clone(&self.queue);
        let config = self.config.clone();
        let reputation = self.reputation.clone();
        let metrics = self.metrics.clone();
        let running = Arc::new(tokio::sync::RwLock::new(true));
        let running_clone = Arc::clone(&running);
        
        let task = tokio::spawn(async move {
            *running_clone.write().await = true;
            
            while let Some(command) = command_rx.recv().await {
                match command {
                    ProcessorCommand::ProcessMessage(envelope) => {
                        // Add the message to the priority queue
                        queue.write().await.push_back(envelope);
                        
                        // Record queue size in metrics
                        if let Some(m) = &metrics {
                            let size = queue.read().await.len();
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
                self.process_queue(
                    &handlers,
                    &queue,
                    &reputation,
                    &metrics,
                ).await;
            }
            
            debug!("Message processor background task stopped");
        });
        
        // Create new clones for the periodic task
        let handlers_periodic = Arc::clone(&self.handlers);
        let queue_periodic = Arc::clone(&self.queue);
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
                    &queue_periodic,
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
        queue: &Arc<RwLock<VecDeque<QueuedMessage>>>,
        reputation: &Option<Arc<ReputationManager>>,
        metrics: &Option<NetworkMetrics>,
    ) {
        let mut processed = 0;
        
        // Process up to MAX_MESSAGES_PER_BATCH messages
        for _ in 0..MAX_MESSAGES_PER_BATCH {
            // Get the highest priority message from the queue
            let envelope = {
                let mut queue_write = queue.write().await;
                if queue_write.is_empty() {
                    break;
                }
                queue_write.pop_front().unwrap()
            };
            
            // Extract message type and peer
            let message_type = envelope.message_type;
            let sender = &envelope.sender;
            
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
                    if let Some(sender_str) = sender {
                        if let Ok(peer_id) = PeerId::from_bytes(sender_str.as_bytes()) {
                            if let Err(e) = rep.record_change(peer_id, ReputationChange::MessageSuccess).await {
                                error!("Failed to update reputation: {}", e);
                            }
                        }
                    }
                }
                
                processed += 1;
            } else {
                debug!("No handlers registered for message type: {}", message_type);
            }
        }
        
        // Update metrics with current queue size
        if let Some(m) = metrics {
            let size = queue.read().await.len();
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
        match self.config.mode {
            PriorityMode::Fifo => {
                // FIFO mode - all messages have the same priority
                0
            },
            PriorityMode::ReputationBased => {
                // Base priority on peer reputation
                let reputation_score = if let Some(rep) = &self.reputation {
                    if let Ok(peer_id) = libp2p::PeerId::from_bytes(peer.peer_id.as_bytes()) {
                        rep.get_reputation(peer_id).await
                            .map(|r| r.score())
                            .unwrap_or(0)
                    } else {
                        0
                    }
                } else {
                    0
                };
                
                // Scale reputation to a priority value
                // Higher reputation = higher priority
                reputation_score
            },
            PriorityMode::TypeAndReputation => {
                // Base priority on message type
                let type_priority = self.config.type_priorities
                    .get(&message.message_type())
                    .copied()
                    .unwrap_or(0);
                
                // Get reputation score
                let reputation_score = if let Some(rep) = &self.reputation {
                    if let Ok(peer_id) = libp2p::PeerId::from_bytes(peer.peer_id.as_bytes()) {
                        rep.get_reputation(peer_id).await
                            .map(|r| r.score())
                            .unwrap_or(0)
                    } else {
                        0
                    }
                } else {
                    0
                };
                
                // Combine type priority and reputation
                // If reputation is high enough, boost priority
                let reputation_boost = if reputation_score >= self.config.high_priority_reputation {
                    20
                } else if reputation_score <= 0 {
                    -10
                } else {
                    0
                };
                
                type_priority + reputation_boost
            },
            PriorityMode::Custom => {
                // Custom prioritization (placeholder)
                // In a real implementation, this would be customizable
                0
            }
        }
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
            peer,
            received_at: Instant::now(),
            priority,
        };
        
        // Check queue size before adding
        let queue_size = self.queue.read().await.len();
        if queue_size >= self.config.max_queue_size {
            if self.config.drop_low_priority_when_full && priority < 0 {
                // Drop low priority messages when queue is full
                debug!("Dropping low priority message (priority: {}) due to full queue", priority);
                
                // Record dropped message in metrics
                if let Some(m) = &self.metrics {
                    m.record_dropped_message();
                }
                
                return Ok(());
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
        self.queue.read().await.len()
    }
    
    /// Get queue statistics
    pub async fn queue_stats(&self) -> (usize, Option<i32>, Option<i32>) {
        let queue = self.queue.read().await;
        let size = queue.len();
        
        // Get highest and lowest priorities if queue is not empty
        let (highest_priority, lowest_priority) = if !queue.is_empty() {
            // Convert the VecDeque to a Vec to access all elements
            let vec: Vec<_> = queue.iter().collect();
            
            // Find highest and lowest priorities
            let highest = vec.iter().map(|e| e.priority).max();
            let lowest = vec.iter().map(|e| e.priority).min();
            
            (highest, lowest)
        } else {
            (None, None)
        };
        
        (size, highest_priority, lowest_priority)
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