use crate::{
    error::{NetworkError, Result},
    tls::TlsConfig,
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

pub struct Node {
    config: NodeConfig,
    tls_config: TlsConfig,
    message_tx: mpsc::Sender<(SocketAddr, Message)>,
    message_rx: mpsc::Receiver<(SocketAddr, Message)>,
    known_peers: HashMap<String, PeerInfo>,
    start_time: std::time::Instant,
}

impl Node {
    pub fn new(config: NodeConfig, tls_config: TlsConfig) -> Self {
        let (message_tx, message_rx) = mpsc::channel(100);
        Self {
            config,
            tls_config,
            message_tx,
            message_rx,
            known_peers: HashMap::new(),
            start_time: std::time::Instant::now(),
        }
    }

    pub async fn start(&mut self) -> Result<()> {
        info!("Starting node {} in cooperative {}", self.config.node_id, self.config.coop_id);
        
        // Start listening for incoming connections
        let listener = TcpListener::bind(self.config.listen_addr).await
            .map_err(|e| NetworkError::Connection(format!("Failed to bind: {}", e)))?;

        info!("Node {} listening on {}", self.config.node_id, self.config.listen_addr);

        // Connect to initial peers
        for &peer_addr in &self.config.peers {
            self.connect_to_peer(peer_addr).await?;
        }

        // Start periodic tasks
        self.start_periodic_tasks();

        // Accept incoming connections
        let acceptor = self.tls_config.acceptor();
        let message_tx = self.message_tx.clone();

        tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, addr)) => {
                        let acceptor = acceptor.clone();
                        let message_tx = message_tx.clone();
                        
                        tokio::spawn(async move {
                            match acceptor.accept(stream).await {
                                Ok(tls_stream) => {
                                    if let Err(e) = handle_connection(tls_stream, addr, message_tx).await {
                                        error!("Error handling connection from {}: {}", addr, e);
                                    }
                                }
                                Err(e) => {
                                    error!("TLS handshake failed with {}: {}", addr, e);
                                }
                            }
                        });
                    }
                    Err(e) => {
                        error!("Failed to accept connection: {}", e);
                    }
                }
            }
        });

        // Process incoming messages
        while let Some((peer_addr, message)) = self.message_rx.recv().await {
            if let Err(e) = self.handle_message(peer_addr, message).await {
                error!("Error handling message from {}: {}", peer_addr, e);
            }
        }

        Ok(())
    }

    fn start_periodic_tasks(&self) {
        // Start discovery task
        if let Some(interval) = self.config.discovery_interval {
            let tx = self.message_tx.clone();
            let node_id = self.config.node_id.clone();
            let coop_id = self.config.coop_id.clone();
            let known_peers = self.known_peers.clone();
            
            tokio::spawn(async move {
                let mut interval = time::interval(interval);
                loop {
                    interval.tick().await;
                    let discovery_msg = Message::Discovery {
                        requesting_node: node_id.clone(),
                        requesting_coop: coop_id.clone(),
                    };
                    
                    // Broadcast discovery message to all known peers
                    for peer in known_peers.values() {
                        if let Err(e) = tx.send((peer.addr, discovery_msg.clone())).await {
                            error!("Failed to send discovery message to {}: {}", peer.addr, e);
                        }
                    }
                }
            });
        }

        // Start health check task
        if let Some(interval) = self.config.health_check_interval {
            let tx = self.message_tx.clone();
            let known_peers = self.known_peers.clone();
            let node_status = self.get_node_status();
            
            tokio::spawn(async move {
                let mut interval = time::interval(interval);
                loop {
                    interval.tick().await;
                    let health_msg = Message::HealthResponse {
                        status: node_status.clone(),
                        metrics: HashMap::new(), // TODO: Implement metrics collection
                    };
                    
                    // Broadcast health status to all known peers
                    for peer in known_peers.values() {
                        if let Err(e) = tx.send((peer.addr, health_msg.clone())).await {
                            error!("Failed to send health status to {}: {}", peer.addr, e);
                        }
                    }
                }
            });
        }
    }

    async fn handle_message(&mut self, peer_addr: SocketAddr, message: Message) -> Result<()> {
        match message {
            Message::Hello { node_id, coop_id, node_type, supported_services: _ } => {
                info!("Received hello from {} ({}) in cooperative {}", node_id, peer_addr, coop_id);
                self.known_peers.insert(node_id.clone(), PeerInfo {
                    node_id,
                    coop_id,
                    node_type,
                    addr: peer_addr,
                    last_seen: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                });
            }
            Message::Discovery { requesting_node, requesting_coop } => {
                info!("Received discovery request from {} in cooperative {}", requesting_node, requesting_coop);
                // Send discovery response with known peers
                let response = Message::DiscoveryResponse {
                    known_peers: self.known_peers.values().cloned().collect(),
                };
                self.message_tx.send((peer_addr, response)).await
                    .map_err(|e| NetworkError::Other(e.to_string()))?;
            }
            Message::DiscoveryResponse { known_peers } => {
                for peer in known_peers {
                    if !self.known_peers.contains_key(&peer.node_id) {
                        self.known_peers.insert(peer.node_id.clone(), peer);
                    }
                }
            }
            Message::HealthCheck => {
                // Respond with current node status
                let response = Message::HealthResponse {
                    status: self.get_node_status(),
                    metrics: HashMap::new(), // TODO: Implement metrics collection
                };
                self.message_tx.send((peer_addr, response)).await
                    .map_err(|e| NetworkError::Other(e.to_string()))?;
            }
            Message::HealthResponse { status, metrics: _ } => {
                info!("Received health status from {}: {:?}", peer_addr, status);
            }
            Message::Data(data) => {
                info!("Received {} bytes from {}", data.len(), peer_addr);
            }
        }
        Ok(())
    }

    fn get_node_status(&self) -> NodeStatus {
        NodeStatus {
            uptime: self.start_time.elapsed().as_secs(),
            connected_peers: self.known_peers.len(),
            cpu_usage: 0.0, // TODO: Implement actual metrics
            memory_usage: 0.0, // TODO: Implement actual metrics
        }
    }

    async fn connect_to_peer(&self, peer_addr: SocketAddr) -> Result<()> {
        let stream = TcpStream::connect(peer_addr).await
            .map_err(|e| NetworkError::Connection(format!("Failed to connect to {}: {}", peer_addr, e)))?;

        let connector = tokio_rustls::TlsConnector::from(self.tls_config.client_config());
        let domain = rustls::pki_types::ServerName::try_from("localhost")
            .map_err(|e| NetworkError::Connection(format!("Invalid server name: {}", e)))?;
            
        let tls_stream = connector
            .connect(domain, stream)
            .await
            .map_err(|e| NetworkError::Connection(format!("TLS handshake failed: {}", e)))?;

        let message_tx = self.message_tx.clone();
        
        tokio::spawn(async move {
            if let Err(e) = handle_connection(tls_stream, peer_addr, message_tx).await {
                eprintln!("Error handling connection to {}: {}", peer_addr, e);
            }
        });

        Ok(())
    }
}

async fn handle_connection<S>(
    mut stream: S,
    peer_addr: SocketAddr,
    message_tx: mpsc::Sender<(SocketAddr, Message)>,
) -> Result<()>
where
    S: AsyncReadExt + AsyncWriteExt + Unpin,
{
    let mut buf = [0u8; 1024];
    
    loop {
        let n = stream.read(&mut buf).await
            .map_err(|e| NetworkError::Io(e))?;
        
        if n == 0 {
            // Connection closed
            return Ok(());
        }
        
        let message: Message = serde_json::from_slice(&buf[..n])
            .map_err(|e| NetworkError::Serialization(e))?;
        
        message_tx.send((peer_addr, message)).await
            .map_err(|e| NetworkError::Other(e.to_string()))?;
    }
} 