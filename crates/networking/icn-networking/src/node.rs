use crate::{
    error::{NetworkError, Result},
    tls::TlsConfig,
    discovery::{DiscoveryService, FederationInfo},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::mpsc,
    time::{self, Duration},
};
use tokio_rustls::TlsAcceptor;
use std::{
    net::SocketAddr,
    sync::Arc,
    collections::HashMap,
    fmt::{self, Display},
};
use serde::{Serialize, Deserialize};
use tracing::{info, warn, error};
use async_trait::async_trait;
use std::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    pub listen_addr: SocketAddr,
    pub peers: Vec<SocketAddr>,
    pub node_id: String,
    pub coop_id: String,
    pub node_type: NodeType,
    pub discovery_interval: Option<Duration>,
    pub health_check_interval: Option<Duration>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NodeType {
    Primary,    // Main node for a cooperative
    Secondary,  // Backup/redundant node
    Edge,       // Edge node for specific services
}

impl Display for NodeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NodeType::Primary => write!(f, "Primary"),
            NodeType::Secondary => write!(f, "Secondary"),
            NodeType::Edge => write!(f, "Edge"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    Hello { 
        node_id: String,
        coop_id: String,
        node_type: NodeType,
        supported_services: Vec<String>,
    },
    Discovery {
        requesting_node: String,
        requesting_coop: String,
    },
    DiscoveryResponse {
        known_peers: Vec<PeerInfo>,
    },
    HealthCheck,
    HealthResponse {
        status: NodeStatus,
        metrics: HashMap<String, f64>,
    },
    Data(Vec<u8>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    pub node_id: String,
    pub coop_id: String,
    pub node_type: NodeType,
    pub addr: SocketAddr,
    pub last_seen: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeStatus {
    pub uptime: u64,
    pub connected_peers: usize,
    pub cpu_usage: f64,
    pub memory_usage: f64,
}

/// Represents a node in the ICN network
pub struct Node {
    /// The node's identifier
    pub id: String,
    
    /// The node's address
    pub address: SocketAddr,
    
    /// The node's TLS configuration
    pub tls_config: TlsConfig,
    
    /// Known peers
    peers: Arc<RwLock<HashMap<String, PeerInfo>>>,
    
    /// Channel for incoming messages
    message_rx: Option<mpsc::Receiver<Message>>,
    
    /// Channel for outgoing messages
    message_tx: Option<mpsc::Sender<Message>>,
    
    /// Discovery service
    discovery_service: Option<DiscoveryService>,
    
    /// Node configuration
    config: NodeConfig,
    
    /// Start time
    start_time: std::time::Instant,
}

/// Information about a peer node
#[derive(Clone, Debug)]
pub struct PeerInfo {
    /// The peer's identifier
    pub id: String,
    
    /// The peer's address
    pub address: SocketAddr,
    
    /// Connection status
    pub status: ConnectionStatus,
    
    /// Last seen timestamp
    pub last_seen: u64,
}

/// Connection status
#[derive(Clone, Debug, PartialEq)]
pub enum ConnectionStatus {
    /// Connected to the peer
    Connected,
    
    /// Disconnected from the peer
    Disconnected,
    
    /// Attempting to connect to the peer
    Connecting,
}

/// A message in the ICN network
#[derive(Clone, Debug)]
pub struct Message {
    /// The sender's identifier
    pub sender: String,
    
    /// The recipient's identifier
    pub recipient: String,
    
    /// The message payload
    pub payload: Vec<u8>,
    
    /// Message type
    pub message_type: MessageType,
}

/// Message types
#[derive(Clone, Debug, PartialEq)]
pub enum MessageType {
    /// Node discovery
    Discovery,
    
    /// Peer exchange
    PeerExchange,
    
    /// Application data
    Data,
}

/// Network service trait
#[async_trait]
pub trait NetworkService: Send + Sync {
    /// Start the network service
    async fn start(&mut self) -> Result<()>;
    
    /// Stop the network service
    async fn stop(&mut self) -> Result<()>;
    
    /// Connect to a peer
    async fn connect(&mut self, address: SocketAddr) -> Result<()>;
    
    /// Disconnect from a peer
    async fn disconnect(&mut self, peer_id: &str) -> Result<()>;
    
    /// Send a message to a peer
    async fn send_message(&self, message: Message) -> Result<()>;
    
    /// Receive a message
    async fn receive_message(&mut self) -> Result<Message>;
    
    /// Get known peers
    fn get_peers(&self) -> Result<Vec<PeerInfo>>;
}

impl Node {
    /// Create a new node
    pub fn new(id: String, address: SocketAddr, tls_config: TlsConfig, config: NodeConfig) -> Self {
        let (tx, rx) = mpsc::channel(100);
        let peers = Arc::new(RwLock::new(HashMap::new()));
        
        // Create initial node
        let mut node = Self {
            id,
            address,
            tls_config,
            peers: peers.clone(),
            message_rx: Some(rx),
            message_tx: Some(tx.clone()),
            discovery_service: None,
            config: config.clone(),
            start_time: std::time::Instant::now(),
        };
        
        // Initialize discovery service
        let bootstrap_nodes = config.peers.clone();
        let discovery_service = DiscoveryService::new(
            config,
            peers,
            bootstrap_nodes,
            tx,
        );
        
        node.discovery_service = Some(discovery_service);
        
        node
    }
    
    /// Add a peer
    pub fn add_peer(&self, peer: PeerInfo) -> Result<()> {
        let mut peers = self.peers.write().map_err(|_| NetworkError::LockError)?;
        peers.insert(peer.id.clone(), peer);
        Ok(())
    }
    
    /// Remove a peer
    pub fn remove_peer(&self, peer_id: &str) -> Result<()> {
        let mut peers = self.peers.write().map_err(|_| NetworkError::LockError)?;
        peers.remove(peer_id);
        Ok(())
    }
    
    /// Get a peer by ID
    pub fn get_peer(&self, peer_id: &str) -> Result<Option<PeerInfo>> {
        let peers = self.peers.read().map_err(|_| NetworkError::LockError)?;
        Ok(peers.get(peer_id).cloned())
    }
    
    /// Get known federations
    pub fn get_federations(&self) -> Result<Vec<FederationInfo>> {
        if let Some(discovery) = &self.discovery_service {
            discovery.get_federations()
        } else {
            Ok(Vec::new())
        }
    }
    
    /// Find a federation by ID
    pub fn find_federation(&self, federation_id: &str) -> Result<Option<FederationInfo>> {
        if let Some(discovery) = &self.discovery_service {
            discovery.find_federation(federation_id)
        } else {
            Ok(None)
        }
    }
    
    /// Announce a federation to the network
    pub async fn announce_federation(
        &self,
        federation_id: String,
        description: String,
        bootstrap_nodes: Vec<SocketAddr>,
        services: Vec<String>,
    ) -> Result<()> {
        if let Some(discovery) = &self.discovery_service {
            discovery.announce_federation(federation_id, description, bootstrap_nodes, services).await
        } else {
            Err(NetworkError::Other("Discovery service not initialized".to_string()))
        }
    }
    
    /// Get node status
    pub fn get_status(&self) -> NodeStatus {
        let uptime = self.start_time.elapsed().as_secs();
        let peers = self.peers.read().unwrap_or_else(|_| panic!("Lock poisoned"));
        
        // In a real implementation, we would collect actual CPU and memory usage
        NodeStatus {
            uptime,
            connected_peers: peers.len(),
            cpu_usage: 0.0,
            memory_usage: 0.0,
        }
    }
}

#[async_trait]
impl NetworkService for Node {
    async fn start(&mut self) -> Result<()> {
        info!("Starting node {} at {}", self.id, self.address);
        
        // Start the discovery service
        if let Some(discovery) = &self.discovery_service {
            discovery.start().await?;
        }
        
        // In a real implementation, we would start a TCP listener
        // and handle connections in a separate task
        // For simplicity, we'll just print that the node has started
        
        // Start a TCP listener
        let listener = TcpListener::bind(self.address).await
            .map_err(|e| NetworkError::Io(e))?;
        
        info!("Node {} listening on {}", self.id, self.address);
        
        // Clone references for the task
        let id = self.id.clone();
        let peers = Arc::clone(&self.peers);
        let message_tx = self.message_tx.clone();
        
        // Spawn a task to handle incoming connections
        tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((socket, addr)) => {
                        info!("Accepted connection from {}", addr);
                        
                        // Here we would handle the connection, authenticate the peer,
                        // and add it to our peer list
                        // For simplicity, we'll just log that a connection was accepted
                        
                        // If we have a message_tx, clone it for this connection
                        if let Some(tx) = &message_tx {
                            let tx = tx.clone();
                            
                            // Spawn a task to handle this connection
                            tokio::spawn(async move {
                                if let Err(e) = handle_connection(socket, addr, id.clone(), tx).await {
                                    error!("Error handling connection from {}: {}", addr, e);
                                }
                            });
                        }
                    }
                    Err(e) => {
                        error!("Error accepting connection: {}", e);
                    }
                }
            }
        });
        
        Ok(())
    }
    
    async fn stop(&mut self) -> Result<()> {
        info!("Stopping node {}", self.id);
        
        // Stop the discovery service
        if let Some(discovery) = &self.discovery_service {
            discovery.stop().await?;
        }
        
        // In a real implementation, we would close all connections
        // and stop the listener
        // For simplicity, we'll just print that the node has stopped
        
        Ok(())
    }
    
    async fn connect(&mut self, address: SocketAddr) -> Result<()> {
        info!("Connecting to peer at {}", address);
        
        // In a real implementation, we would establish a TLS connection
        // and exchange node information
        // For simplicity, we'll just print that we're connecting
        
        Ok(())
    }
    
    async fn disconnect(&mut self, peer_id: &str) -> Result<()> {
        info!("Disconnecting from peer {}", peer_id);
        
        // In a real implementation, we would close the connection
        // and update the peer's status
        self.remove_peer(peer_id)?;
        
        Ok(())
    }
    
    async fn send_message(&self, message: Message) -> Result<()> {
        info!("Sending message to {}: {:?}", message.recipient, message.message_type);
        
        // In a real implementation, we would serialize the message
        // and send it over the appropriate connection
        // For simplicity, we'll just print that we're sending a message
        
        Ok(())
    }
    
    async fn receive_message(&mut self) -> Result<Message> {
        // In a real implementation, we would receive a message from
        // the message channel
        let rx = self.message_rx.as_mut().ok_or(NetworkError::ChannelClosed)?;
        rx.recv().await.ok_or(NetworkError::ChannelClosed.into())
    }
    
    fn get_peers(&self) -> Result<Vec<PeerInfo>> {
        let peers = self.peers.read().map_err(|_| NetworkError::LockError)?;
        Ok(peers.values().cloned().collect())
    }
}

/// Handle a new connection
async fn handle_connection(
    mut socket: TcpStream,
    addr: SocketAddr,
    node_id: String,
    tx: mpsc::Sender<Message>,
) -> Result<()> {
    // In a real implementation, we would:
    // 1. Perform TLS handshake
    // 2. Read the initial Hello message
    // 3. Verify the peer
    // 4. Add the peer to our peer list
    // 5. Handle messages from the peer
    
    // For simplicity, we'll just print that we're handling a connection
    info!("Handling connection from {}", addr);
    
    // Let's simulate reading and responding to messages
    loop {
        // Buffer for incoming data
        let mut buffer = [0u8; 1024];
        
        // Read data from the socket
        let n = match socket.read(&mut buffer).await {
            Ok(0) => {
                // Connection closed
                info!("Connection from {} closed", addr);
                break;
            }
            Ok(n) => n,
            Err(e) => {
                error!("Error reading from socket: {}", e);
                return Err(NetworkError::Io(e));
            }
        };
        
        // For simplicity, we'll just echo the data back
        if let Err(e) = socket.write_all(&buffer[0..n]).await {
            error!("Error writing to socket: {}", e);
            return Err(NetworkError::Io(e));
        }
    }
    
    Ok(())
}

/// Node discovery service
pub struct DiscoveryService {
    /// The node
    pub node: Arc<Node>,
    
    /// Known bootstrap nodes
    pub bootstrap_nodes: Vec<SocketAddr>,
    
    /// Discovery interval in seconds
    pub discovery_interval: u64,
}

impl DiscoveryService {
    /// Create a new discovery service
    pub fn new(node: Arc<Node>, bootstrap_nodes: Vec<SocketAddr>, discovery_interval: u64) -> Self {
        Self {
            node,
            bootstrap_nodes,
            discovery_interval,
        }
    }
    
    /// Start the discovery service
    pub async fn start(&self) -> Result<()> {
        // Placeholder implementation
        // In a real implementation, we would periodically send discovery
        // messages to bootstrap nodes and process responses
        for &addr in &self.bootstrap_nodes {
            println!("Discovering nodes through bootstrap node at {}", addr);
        }
        
        Ok(())
    }
    
    /// Stop the discovery service
    pub async fn stop(&self) -> Result<()> {
        // Placeholder implementation
        Ok(())
    }
    
    /// Process a discovery message
    pub async fn process_discovery_message(&self, message: Message) -> Result<()> {
        // Placeholder implementation
        // In a real implementation, we would extract peer information
        // from the message and add the peers to our known peers
        println!("Processing discovery message from {}", message.sender);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_add_peer() {
        let config = NodeConfig {
            listen_addr: "127.0.0.1:8000".parse().unwrap(),
            peers: vec![],
            node_id: "node1".to_string(),
            coop_id: "coop1".to_string(),
            node_type: NodeType::Primary,
            discovery_interval: None,
            health_check_interval: None,
        };

        let node = Node::new(
            "node1".to_string(),
            "127.0.0.1:8000".parse().unwrap(),
            TlsConfig::default(),
            config,
        );
        
        let peer = PeerInfo {
            id: "peer1".to_string(),
            address: "127.0.0.1:8001".parse().unwrap(),
            status: ConnectionStatus::Disconnected,
            last_seen: 0,
        };
        
        node.add_peer(peer.clone()).unwrap();
        
        let retrieved_peer = node.get_peer(&peer.id).unwrap().unwrap();
        assert_eq!(retrieved_peer.id, peer.id);
        assert_eq!(retrieved_peer.address, peer.address);
        assert_eq!(retrieved_peer.status, peer.status);
    }
} 