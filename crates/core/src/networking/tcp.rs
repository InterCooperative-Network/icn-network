//! TCP-based network implementation
//!
//! This module provides a TCP-based implementation of the Network trait.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};
use async_trait::async_trait;

use super::{Network, NetworkError, NetworkMessage, NetworkResult};

/// Configuration for TCP network
#[derive(Debug, Clone)]
pub struct TcpNetworkConfig {
    /// Local address to bind to
    pub bind_address: SocketAddr,
    /// Maximum number of connection attempts
    pub max_connection_attempts: u32,
    /// Connection timeout in seconds
    pub connection_timeout: u64,
    /// Heartbeat interval in seconds
    pub heartbeat_interval: u64,
}

impl Default for TcpNetworkConfig {
    fn default() -> Self {
        Self {
            bind_address: "127.0.0.1:9000".parse().unwrap(),
            max_connection_attempts: 3,
            connection_timeout: 5,
            heartbeat_interval: 30,
        }
    }
}

type MessageHandler = Box<dyn Fn(NetworkMessage) -> NetworkResult<()> + Send + Sync + 'static>;

/// TCP implementation of the Network trait
pub struct TcpNetwork {
    /// Node ID for this network instance
    node_id: String,
    /// Network configuration
    config: TcpNetworkConfig,
    /// Connected peers (peer_id -> connection)
    peers: Arc<RwLock<HashMap<String, TcpPeer>>>,
    /// Message handlers (message_type -> handler)
    handlers: Arc<RwLock<HashMap<String, MessageHandler>>>,
    /// Server task handle
    server_handle: Arc<RwLock<Option<JoinHandle<()>>>>,
    /// Running state
    running: Arc<RwLock<bool>>,
}

/// Represents a peer connection
struct TcpPeer {
    /// Peer ID
    id: String,
    /// Socket address
    address: SocketAddr,
    /// Connection status
    connected: bool,
    /// Stream handle
    stream: Option<Arc<RwLock<TcpStream>>>,
    /// Task handle for the peer handler
    task_handle: Option<JoinHandle<()>>,
}

impl TcpNetwork {
    /// Create a new TCP network instance
    pub fn new(node_id: impl Into<String>, config: TcpNetworkConfig) -> Self {
        Self {
            node_id: node_id.into(),
            config,
            peers: Arc::new(RwLock::new(HashMap::new())),
            handlers: Arc::new(RwLock::new(HashMap::new())),
            server_handle: Arc::new(RwLock::new(None)),
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Start the TCP server listener
    async fn start_server(&self) -> NetworkResult<JoinHandle<()>> {
        let listener = TcpListener::bind(self.config.bind_address).await
            .map_err(|e| NetworkError::IoError(e))?;
        
        info!("TCP Network server listening on {}", self.config.bind_address);
        
        let peers = Arc::clone(&self.peers);
        let handlers = Arc::clone(&self.handlers);
        let running = Arc::clone(&self.running);
        let node_id = self.node_id.clone();
        
        // Spawn listener task
        let handle = tokio::spawn(async move {
            while *running.read().await {
                match listener.accept().await {
                    Ok((stream, addr)) => {
                        debug!("Accepted connection from {}", addr);
                        // Handle new connection
                        // In a real implementation, we would handle authentication and peer registration
                        // For now, just log and drop the connection
                    }
                    Err(e) => {
                        error!("Error accepting connection: {}", e);
                    }
                }
            }
        });
        
        Ok(handle)
    }
}

#[async_trait]
impl Network for TcpNetwork {
    async fn start(&self) -> NetworkResult<()> {
        let mut running = self.running.write().await;
        if *running {
            return Ok(());
        }
        
        *running = true;
        
        // Start TCP server
        let server_handle = self.start_server().await?;
        
        // Store server handle
        let mut handle = self.server_handle.write().await;
        *handle = Some(server_handle);
        
        info!("TCP Network started with node ID: {}", self.node_id);
        Ok(())
    }
    
    async fn stop(&self) -> NetworkResult<()> {
        let mut running = self.running.write().await;
        if !*running {
            return Ok(());
        }
        
        *running = false;
        
        // Stop server
        let mut server_handle = self.server_handle.write().await;
        if let Some(handle) = server_handle.take() {
            handle.abort();
        }
        
        // Disconnect all peers
        let mut peers = self.peers.write().await;
        for (peer_id, peer) in peers.iter_mut() {
            if let Some(handle) = peer.task_handle.take() {
                handle.abort();
            }
            peer.connected = false;
            peer.stream = None;
        }
        
        info!("TCP Network stopped");
        Ok(())
    }
    
    async fn connect(&self, address: SocketAddr) -> NetworkResult<()> {
        // Not fully implemented for this example
        // In a real implementation, we would:
        // 1. Establish a TCP connection
        // 2. Handle authentication
        // 3. Register the peer
        
        info!("Connecting to peer at {}", address);
        Ok(())
    }
    
    async fn disconnect(&self, peer_id: &str) -> NetworkResult<()> {
        let mut peers = self.peers.write().await;
        if let Some(peer) = peers.get_mut(peer_id) {
            if let Some(handle) = peer.task_handle.take() {
                handle.abort();
            }
            peer.connected = false;
            peer.stream = None;
            
            info!("Disconnected from peer: {}", peer_id);
        }
        Ok(())
    }
    
    async fn send_to(&self, peer_id: &str, message: NetworkMessage) -> NetworkResult<()> {
        // Not fully implemented for this example
        debug!("Sending message to {}: {:?}", peer_id, message);
        Ok(())
    }
    
    async fn broadcast(&self, message: NetworkMessage) -> NetworkResult<()> {
        let peers = self.peers.read().await;
        for (peer_id, _) in peers.iter() {
            self.send_to(peer_id, message.clone()).await?;
        }
        Ok(())
    }
    
    async fn register_handler<F>(&self, message_type: &str, handler: F) -> NetworkResult<()>
    where
        F: Fn(NetworkMessage) -> NetworkResult<()> + Send + Sync + 'static,
    {
        let mut handlers = self.handlers.write().await;
        handlers.insert(message_type.to_string(), Box::new(handler));
        Ok(())
    }
    
    async fn get_peers(&self) -> NetworkResult<Vec<String>> {
        let peers = self.peers.read().await;
        Ok(peers.keys().cloned().collect())
    }
} 