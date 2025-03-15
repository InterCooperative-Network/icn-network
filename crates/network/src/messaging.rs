//! Network messaging module for ICN
//!
//! This module handles message encoding, decoding, and processing 
//! for communication between nodes in the InterCooperative Network.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use serde::{Serialize, Deserialize};

use crate::{NetworkError, NetworkResult, NetworkMessage, MessageHandler, PeerInfo};

/// Message envelope containing the sender information and the actual message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageEnvelope {
    /// The sender's peer ID as a string
    pub sender: String,
    /// The message type identifier
    pub message_type: String,
    /// The serialized message payload
    pub payload: Vec<u8>,
    /// Message timestamp (unix timestamp in milliseconds)
    pub timestamp: u64,
    /// Optional signature of the message payload
    pub signature: Option<Vec<u8>>,
}

impl MessageEnvelope {
    /// Create a new message envelope
    pub fn new(
        sender: String,
        message_type: String,
        payload: Vec<u8>,
        timestamp: u64,
        signature: Option<Vec<u8>>,
    ) -> Self {
        Self {
            sender,
            message_type,
            payload,
            timestamp,
            signature,
        }
    }
    
    /// Serialize the envelope to bytes
    pub fn to_bytes(&self) -> NetworkResult<Vec<u8>> {
        serde_json::to_vec(self)
            .map_err(|e| NetworkError::SerializationError(e.to_string()))
    }
    
    /// Deserialize bytes to a message envelope
    pub fn from_bytes(bytes: &[u8]) -> NetworkResult<Self> {
        serde_json::from_slice(bytes)
            .map_err(|e| NetworkError::DeserializationError(e.to_string()))
    }
}

/// Message processor for handling incoming messages
pub struct MessageProcessor {
    /// Registered message handlers
    handlers: Arc<RwLock<HashMap<String, Vec<Arc<dyn MessageHandler>>>>>,
}

impl MessageProcessor {
    /// Create a new message processor
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Register a handler for a specific message type
    pub async fn register_handler(&self, message_type: &str, handler: Arc<dyn MessageHandler>) {
        let mut handlers = self.handlers.write().await;
        
        let type_handlers = handlers.entry(message_type.to_string()).or_insert_with(Vec::new);
        type_handlers.push(handler);
        
        debug!("Registered handler for message type: {}", message_type);
    }
    
    /// Unregister a handler
    pub async fn unregister_handler(&self, message_type: &str, handler_id: usize) -> bool {
        let mut handlers = self.handlers.write().await;
        
        if let Some(type_handlers) = handlers.get_mut(message_type) {
            // Find the handler by its ID
            if let Some(pos) = type_handlers.iter().position(|h| h.id() == handler_id) {
                type_handlers.remove(pos);
                debug!("Unregistered handler {} for message type: {}", handler_id, message_type);
                return true;
            }
        }
        
        false
    }
    
    /// Process an incoming message envelope
    pub async fn process_message(&self, envelope: &MessageEnvelope, peer: &PeerInfo) -> NetworkResult<()> {
        let message_type = &envelope.message_type;
        
        // Check if we have handlers for this message type
        let handlers = {
            let handlers_map = self.handlers.read().await;
            if let Some(type_handlers) = handlers_map.get(message_type) {
                type_handlers.clone()
            } else {
                debug!("No handlers registered for message type: {}", message_type);
                return Ok(());
            }
        };
        
        // Create the network message from the envelope
        let message = self.decode_message(envelope).await?;
        
        // Call each handler
        for handler in handlers {
            match handler.handle_message(&message, peer).await {
                Ok(_) => {
                    debug!("Handler {} processed message of type {}", handler.id(), message_type);
                }
                Err(e) => {
                    warn!("Handler {} failed to process message of type {}: {}", 
                         handler.id(), message_type, e);
                }
            }
        }
        
        Ok(())
    }
    
    /// Encode a network message into an envelope
    pub async fn encode_message(&self, 
                               sender: String,
                               message: &NetworkMessage,
                               signature: Option<Vec<u8>>) -> NetworkResult<MessageEnvelope> {
        // Get the message type string
        let message_type = match message {
            NetworkMessage::IdentityAnnouncement(_) => "identity.announcement",
            NetworkMessage::TransactionAnnouncement(_) => "ledger.transaction",
            NetworkMessage::LedgerStateUpdate(_) => "ledger.state",
            NetworkMessage::ProposalAnnouncement(_) => "governance.proposal",
            NetworkMessage::VoteAnnouncement(_) => "governance.vote",
            NetworkMessage::Custom(custom) => custom.message_type.as_str(),
        };
        
        // Serialize the message
        let payload = serde_json::to_vec(message)
            .map_err(|e| NetworkError::SerializationError(e.to_string()))?;
        
        // Create the timestamp
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        
        // Create the envelope
        let envelope = MessageEnvelope::new(
            sender,
            message_type.to_string(),
            payload,
            timestamp,
            signature,
        );
        
        Ok(envelope)
    }
    
    /// Decode a message envelope into a network message
    pub async fn decode_message(&self, envelope: &MessageEnvelope) -> NetworkResult<NetworkMessage> {
        let message_type = &envelope.message_type;
        let payload = &envelope.payload;
        
        // Deserialize based on the message type
        match message_type.as_str() {
            "identity.announcement" => {
                let msg = serde_json::from_slice(payload)
                    .map_err(|e| NetworkError::DeserializationError(format!(
                        "Failed to deserialize identity announcement: {}", e
                    )))?;
                Ok(NetworkMessage::IdentityAnnouncement(msg))
            },
            "ledger.transaction" => {
                let msg = serde_json::from_slice(payload)
                    .map_err(|e| NetworkError::DeserializationError(format!(
                        "Failed to deserialize transaction announcement: {}", e
                    )))?;
                Ok(NetworkMessage::TransactionAnnouncement(msg))
            },
            "ledger.state" => {
                let msg = serde_json::from_slice(payload)
                    .map_err(|e| NetworkError::DeserializationError(format!(
                        "Failed to deserialize ledger state update: {}", e
                    )))?;
                Ok(NetworkMessage::LedgerStateUpdate(msg))
            },
            "governance.proposal" => {
                let msg = serde_json::from_slice(payload)
                    .map_err(|e| NetworkError::DeserializationError(format!(
                        "Failed to deserialize proposal announcement: {}", e
                    )))?;
                Ok(NetworkMessage::ProposalAnnouncement(msg))
            },
            "governance.vote" => {
                let msg = serde_json::from_slice(payload)
                    .map_err(|e| NetworkError::DeserializationError(format!(
                        "Failed to deserialize vote announcement: {}", e
                    )))?;
                Ok(NetworkMessage::VoteAnnouncement(msg))
            },
            // For custom messages
            _ => {
                // Attempt to deserialize as a custom message
                let custom = serde_json::from_slice(payload)
                    .map_err(|e| NetworkError::DeserializationError(format!(
                        "Failed to deserialize custom message of type {}: {}", message_type, e
                    )))?;
                Ok(NetworkMessage::Custom(custom))
            }
        }
    }
}

/// Default implementation of a message handler that delegates to closures
pub struct DefaultMessageHandler {
    /// Handler ID
    id: usize,
    /// Handler name
    name: String,
    /// The actual handler function
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
    /// Get the handler ID
    fn id(&self) -> usize {
        self.id
    }
    
    /// Get the handler name
    fn name(&self) -> &str {
        &self.name
    }
    
    /// Handle a received message
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