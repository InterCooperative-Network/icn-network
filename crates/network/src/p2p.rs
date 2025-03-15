//! P2P network implementation using libp2p
//!
//! This module provides the core implementation of the P2P network
//! functionality for the ICN.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::time::Instant;

use async_trait::async_trait;
use futures::prelude::*;
use libp2p::{
    core::{muxing::StreamMuxerBox, transport::OrTransport, upgrade},
    gossipsub::{self, IdentTopic, MessageAuthenticity, MessageId, ValidationMode},
    identify, kad, mdns, noise, ping, swarm,
    swarm::{NetworkBehaviour, SwarmBuilder, SwarmEvent},
    tcp, yamux, Multiaddr, PeerId,
    identity::Keypair,
};
use tokio::sync::{mpsc, RwLock, Mutex};
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

use icn_core::storage::Storage;

use crate::{
    MessageHandler, NetworkError, NetworkMessage, NetworkResult, NetworkService,
    PeerInfo,
};
use crate::metrics::NetworkMetrics;
use crate::reputation::{ReputationManager, ReputationConfig, ReputationChange};
use crate::messaging;
use crate::circuit_relay::{CircuitRelayConfig, CircuitRelayManager, configure_relay_transport};

// Topic names for gossipsub
const TOPIC_IDENTITY: &str = "icn/identity/v1";
const TOPIC_TRANSACTIONS: &str = "icn/transactions/v1";
const TOPIC_LEDGER: &str = "icn/ledger/v1";
const TOPIC_GOVERNANCE: &str = "icn/governance/v1";

/// Configuration for the P2P network
#[derive(Debug, Clone)]
pub struct P2pConfig {
    /// Local listening addresses
    pub listen_addresses: Vec<Multiaddr>,
    /// Bootstrap peers
    pub bootstrap_peers: Vec<String>,
    /// Enable mDNS discovery
    pub enable_mdns: bool,
    /// Enable Kademlia DHT
    pub enable_kademlia: bool,
    /// Gossipsub message validation mode
    pub gossipsub_validation: ValidationMode,
    /// Message validation timeout
    pub message_timeout: Duration,
    /// Connection keep alive timeout
    pub keep_alive: Duration,
    /// Path to persistent peer storage
    pub peer_store_path: Option<String>,
    /// Enable metrics collection
    pub enable_metrics: bool,
    /// Metrics server address
    pub metrics_address: Option<String>,
    /// Enable reputation system
    pub enable_reputation: bool,
    /// Configuration for the reputation system
    pub reputation_config: Option<ReputationConfig>,
    /// Enable message prioritization
    pub enable_message_prioritization: bool,
    /// Priority configuration
    pub priority_config: Option<messaging::PriorityConfig>,
    /// Enable circuit relay
    pub enable_circuit_relay: bool,
    /// Circuit relay configuration
    pub circuit_relay_config: Option<CircuitRelayConfig>,
}

impl Default for P2pConfig {
    fn default() -> Self {
        Self {
            listen_addresses: vec!["/ip4/0.0.0.0/tcp/0".parse().unwrap()],
            bootstrap_peers: Vec::new(),
            enable_mdns: true,
            enable_kademlia: true,
            gossipsub_validation: ValidationMode::Strict,
            message_timeout: Duration::from_secs(10),
            keep_alive: Duration::from_secs(120),
            peer_store_path: None,
            enable_metrics: false,
            metrics_address: None,
            enable_reputation: false,
            reputation_config: None,
            enable_message_prioritization: true,
            priority_config: None,
            enable_circuit_relay: false,
            circuit_relay_config: None,
        }
    }
}

/// Network behavior combining multiple protocols
#[derive(NetworkBehaviour)]
#[behaviour(out_event = "ComposedEvent")]
struct P2pBehaviour {
    /// Ping protocol for measuring latency
    ping: ping::Behaviour,
    /// Identify protocol for exchanging node information
    identify: identify::Behaviour,
    /// Kademlia DHT for peer discovery
    kad: kad::Behaviour<kad::store::MemoryStore>,
    /// mDNS for local network discovery
    mdns: mdns::async_io::Behaviour,
    /// GossipSub for efficient message propagation
    gossipsub: gossipsub::Behaviour,
}

/// Combined events from all protocols
#[derive(Debug)]
enum ComposedEvent {
    /// Ping events
    Ping(ping::Event),
    /// Identify events
    Identify(identify::Event),
    /// Kademlia events
    Kad(kad::Event),
    /// mDNS events
    Mdns(mdns::Event),
    /// GossipSub events
    Gossipsub(gossipsub::Event),
}

impl From<ping::Event> for ComposedEvent {
    fn from(event: ping::Event) -> Self {
        ComposedEvent::Ping(event)
    }
}

impl From<identify::Event> for ComposedEvent {
    fn from(event: identify::Event) -> Self {
        ComposedEvent::Identify(event)
    }
}

impl From<kad::Event> for ComposedEvent {
    fn from(event: kad::Event) -> Self {
        ComposedEvent::Kad(event)
    }
}

impl From<mdns::Event> for ComposedEvent {
    fn from(event: mdns::Event) -> Self {
        ComposedEvent::Mdns(event)
    }
}

impl From<gossipsub::Event> for ComposedEvent {
    fn from(event: gossipsub::Event) -> Self {
        ComposedEvent::Gossipsub(event)
    }
}

/// Command messages to control the network service
enum Command {
    /// Broadcast a message to all peers
    Broadcast(NetworkMessage),
    /// Send a message to a specific peer
    SendTo(PeerId, NetworkMessage),
    /// Connect to a peer
    Connect(Multiaddr, mpsc::Sender<NetworkResult<PeerId>>),
    /// Disconnect from a peer
    Disconnect(PeerId, mpsc::Sender<NetworkResult<()>>),
    /// Get information about a peer
    GetPeerInfo(PeerId, mpsc::Sender<NetworkResult<PeerInfo>>),
    /// Get a list of connected peers
    GetConnectedPeers(mpsc::Sender<NetworkResult<Vec<PeerInfo>>>),
    /// Register message handler
    RegisterHandler(String, Arc<dyn MessageHandler>, mpsc::Sender<NetworkResult<()>>),
    /// Get listen addresses
    GetListenAddresses(mpsc::Sender<NetworkResult<Vec<Multiaddr>>>),
    /// Stop the network service
    Stop(mpsc::Sender<NetworkResult<()>>),
}

/// The main P2P network service implementation
pub struct P2pNetwork {
    /// Storage for network data
    storage: Arc<dyn Storage>,
    /// libp2p key pair
    key_pair: Keypair,
    /// Local peer ID
    local_peer_id: PeerId,
    /// Network configuration
    config: P2pConfig,
    /// Command sender
    command_tx: mpsc::Sender<Command>,
    /// Background task handle
    task_handle: Arc<Mutex<Option<JoinHandle<()>>>>,
    /// Message handlers
    handlers: Arc<RwLock<HashMap<String, Vec<Arc<dyn MessageHandler>>>>>,
    /// Known peers
    peers: Arc<RwLock<HashMap<PeerId, PeerInfo>>>,
    /// Network metrics
    metrics: Option<NetworkMetrics>,
    /// Reputation manager
    reputation: Option<Arc<ReputationManager>>,
    /// Message processor for prioritized handling
    message_processor: Option<Arc<messaging::MessageProcessor>>,
    /// Circuit relay manager
    circuit_relay: Option<Arc<CircuitRelayManager>>,
}

impl P2pNetwork {
    /// Create a new P2P network
    pub async fn new(
        storage: Arc<dyn Storage>,
        config: P2pConfig,
    ) -> NetworkResult<Self> {
        // Generate or load keypair
        let key_pair = Self::load_or_create_keypair(storage.clone()).await?;
        let local_peer_id = PeerId::from(key_pair.public());
        
        debug!("Local peer ID: {}", local_peer_id);
        
        // Create message handlers map
        let handlers = Arc::new(RwLock::new(HashMap::new()));
        
        // Create peer info map
        let peers = Arc::new(RwLock::new(HashMap::new()));
        
        // Create command channel
        let (command_tx, command_rx) = mpsc::channel(100);
        
        // Create metrics if enabled
        let metrics = if config.enable_metrics {
            let metrics = NetworkMetrics::new();
            
            // Start metrics server if address is provided
            if let Some(addr) = &config.metrics_address {
                start_metrics_server(metrics.clone(), addr).await?;
            }
            
            Some(metrics)
        } else {
            None
        };
        
        // Create reputation manager if enabled
        let reputation = if config.enable_reputation {
            let rep_config = config.reputation_config.clone().unwrap_or_default();
            let manager = ReputationManager::new(rep_config, metrics.clone());
            
            // Start decay task
            manager.start_decay_task();
            
            Some(Arc::new(manager))
        } else {
            None
        };
        
        // Create message processor if prioritization is enabled
        let message_processor = if config.enable_message_prioritization {
            let priority_config = config.priority_config.clone().unwrap_or_default();
            let processor = messaging::MessageProcessor::new(
                handlers.clone(),
                priority_config,
                reputation.clone(),
                metrics.clone(),
            );
            
            Some(Arc::new(processor))
        } else {
            None
        };
        
        // Create circuit relay manager if enabled
        let circuit_relay = if config.enable_circuit_relay {
            let relay_config = config.circuit_relay_config.clone().unwrap_or_default();
            let manager = CircuitRelayManager::new(relay_config, metrics.clone());
            
            // Initialize relay manager
            manager.initialize().await?;
            
            // Start cleanup task
            manager.start_cleanup_task();
            
            Some(Arc::new(manager))
        } else {
            None
        };
        
        // Create network instance
        let network = Self {
            storage,
            key_pair,
            local_peer_id,
            config,
            command_tx,
            task_handle: Arc::new(Mutex::new(None)),
            handlers,
            peers,
            metrics,
            reputation,
            message_processor,
            circuit_relay,
        };
        
        // Start background task
        network.start_background_task(command_rx).await?;
        
        Ok(network)
    }
    
    /// Get the local peer ID
    pub fn local_peer_id(&self) -> PeerId {
        self.local_peer_id
    }
    
    /// Get the listen addresses
    pub async fn listen_addresses(&self) -> NetworkResult<Vec<Multiaddr>> {
        let (tx, rx) = mpsc::channel(1);
        self.command_tx.send(Command::GetListenAddresses(tx)).await
            .map_err(|_| NetworkError::InternalError("Failed to send get_listen_addresses command".to_string()))?;
        
        // Wait for the response
        rx.await
            .map_err(|_| NetworkError::InternalError("Failed to receive get_listen_addresses response".to_string()))?
    }
    
    /// Load an existing or create a new libp2p keypair
    async fn load_or_create_keypair(storage: Arc<dyn Storage>) -> NetworkResult<Keypair> {
        // Try to load from storage
        let key_path = "network/libp2p_key";
        if storage.exists(key_path).await? {
            match storage.get(key_path).await {
                Ok(bytes) => {
                    // Try to deserialize the key
                    match Keypair::from_protobuf_encoding(&bytes) {
                        Ok(keypair) => return Ok(keypair),
                        Err(e) => {
                            warn!("Failed to deserialize keypair: {}", e);
                            // Continue and generate a new one
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to load keypair: {}", e);
                    // Continue and generate a new one
                }
            }
        }
        
        // Generate a new keypair
        let keypair = Keypair::generate_ed25519();
        
        // Save it for future use
        let bytes = keypair.to_protobuf_encoding()
            .map_err(|e| NetworkError::Libp2pError(e.to_string()))?;
        storage.put(key_path, &bytes).await?;
        
        Ok(keypair)
    }
    
    /// Create the swarm with all network behaviors
    fn create_swarm(key_pair: &Keypair, config: &P2pConfig) -> NetworkResult<swarm::Swarm<P2pBehaviour>> {
        let local_peer_id = PeerId::from(key_pair.public());
        
        // Create transport
        let base_transport = {
            let tcp = libp2p::tcp::tokio::Transport::default()
                .upgrade(upgrade::Version::V1)
                .authenticate(noise::Config::new(key_pair)?)
                .multiplex(yamux::Config::default())
                .timeout(config.keep_alive);
            
            tcp.boxed()
        };
        
        // Add circuit relay transport if enabled
        let transport = if config.enable_circuit_relay {
            let relay_config = config.circuit_relay_config.clone().unwrap_or_default();
            configure_relay_transport(base_transport, local_peer_id, &relay_config)?
        } else {
            base_transport
        };
        
        // Set up gossipsub
        let gossipsub_config = gossipsub::ConfigBuilder::default()
            .heartbeat_interval(Duration::from_secs(10))
            .validation_mode(config.gossipsub_validation.clone())
            .build()
            .map_err(|e| NetworkError::Libp2pError(e.to_string()))?;
        
        let message_authenticity = MessageAuthenticity::Signed(key_pair.clone());
        let mut gossipsub = gossipsub::Behaviour::new(message_authenticity, gossipsub_config)
            .map_err(|e| NetworkError::Libp2pError(e.to_string()))?;
        
        // Subscribe to topics
        let topic_identity = IdentTopic::new(TOPIC_IDENTITY);
        let topic_transactions = IdentTopic::new(TOPIC_TRANSACTIONS);
        let topic_ledger = IdentTopic::new(TOPIC_LEDGER);
        let topic_governance = IdentTopic::new(TOPIC_GOVERNANCE);
        
        gossipsub.subscribe(&topic_identity)
            .map_err(|e| NetworkError::Libp2pError(e.to_string()))?;
        gossipsub.subscribe(&topic_transactions)
            .map_err(|e| NetworkError::Libp2pError(e.to_string()))?;
        gossipsub.subscribe(&topic_ledger)
            .map_err(|e| NetworkError::Libp2pError(e.to_string()))?;
        gossipsub.subscribe(&topic_governance)
            .map_err(|e| NetworkError::Libp2pError(e.to_string()))?;
        
        // Set up Kademlia
        let store = kad::store::MemoryStore::new(key_pair.public().to_peer_id());
        let kad_config = kad::Config::default();
        let kad_behaviour = kad::Behaviour::with_config(
            key_pair.public().to_peer_id(),
            store,
            kad_config,
        );
        
        // Set up mDNS
        let mdns = mdns::async_io::Behaviour::new(mdns::Config::default(), key_pair.public().to_peer_id())
            .map_err(|e| NetworkError::Libp2pError(e.to_string()))?;
        
        // Build the swarm
        let behaviour = P2pBehaviour {
            ping: ping::Behaviour::new(ping::Config::new()),
            identify: identify::Behaviour::new(identify::Config::new(
                "/ipfs/id/1.0.0".to_string(),
                key_pair.public(),
            )),
            kad: kad_behaviour,
            mdns,
            gossipsub,
        };
        
        let swarm = SwarmBuilder::with_tokio_executor(
            transport,
            behaviour,
            key_pair.public().to_peer_id(),
        ).build();
        
        Ok(swarm)
    }
    
    /// Start the background network task
    async fn start_background_task(&self, mut command_rx: mpsc::Receiver<Command>) -> NetworkResult<()> {
        let mut swarm = Self::create_swarm(&self.key_pair, &self.config)?;
        
        // Listen on configured addresses
        for addr in &self.config.listen_addresses {
            swarm.listen_on(addr.clone())
                .map_err(|e| NetworkError::Libp2pError(format!("Failed to listen on {}: {}", addr, e)))?;
        }
        
        // Connect to bootstrap peers
        for addr in &self.config.bootstrap_peers {
            match swarm.dial(addr.parse::<Multiaddr>()?) {
                Ok(_) => info!("Dialing bootstrap peer {}", addr),
                Err(e) => warn!("Failed to dial bootstrap peer {}: {}", addr, e),
            }
        }
        
        // Clone needed components for the task
        let handlers = self.handlers.clone();
        let peers = self.peers.clone();
        let peer_id = self.local_peer_id;
        
        // Create a channel for listen addresses
        let (listen_tx, mut listen_rx) = mpsc::channel::<Multiaddr>(16);
        let listen_addresses = Arc::new(RwLock::new(Vec::<Multiaddr>::new()));
        let listen_addresses_clone = listen_addresses.clone();
        
        // Spawn a task to collect listen addresses
        tokio::spawn(async move {
            while let Some(addr) = listen_rx.recv().await {
                let mut addrs = listen_addresses_clone.write().await;
                if !addrs.contains(&addr) {
                    addrs.push(addr);
                }
            }
        });
        
        // Record metrics for initial state
        if let Some(metrics) = &self.metrics {
            let peer_count = swarm.connected_peers().count();
            metrics.peers_connected.set(peer_count as i64);
        }
        
        // Start the network task
        let task = tokio::spawn(async move {
            info!("P2P network task started, peer ID: {}", peer_id);
            
            loop {
                tokio::select! {
                    event = swarm.select_next_some() => {
                        let start_time = Instant::now();
                        
                        Self::handle_swarm_event(
                            event, 
                            &mut swarm, 
                            &handlers,
                            &peers,
                            self.metrics.as_ref(),
                            self.reputation.as_ref(),
                            self.message_processor.as_ref(),
                        ).await;
                        
                        // Record event processing time
                        if let Some(metrics) = &self.metrics {
                            let elapsed = start_time.elapsed();
                            metrics.record_message_processing_time(elapsed);
                        }
                    }
                    cmd = command_rx.recv() => {
                        match cmd {
                            Some(Command::Broadcast(message)) => {
                                let start_time = Instant::now();
                                
                                Self::handle_broadcast(&mut swarm, message.clone()).await;
                                
                                // Record metrics
                                if let Some(metrics) = &self.metrics {
                                    let elapsed = start_time.elapsed();
                                    metrics.record_message_processing_time(elapsed);
                                    
                                    // Record message size approximately
                                    let message_type = match &message {
                                        NetworkMessage::IdentityAnnouncement(_) => "identity",
                                        NetworkMessage::TransactionAnnouncement(_) => "transaction",
                                        NetworkMessage::LedgerStateUpdate(_) => "ledger",
                                        NetworkMessage::ProposalAnnouncement(_) => "proposal",
                                        NetworkMessage::VoteAnnouncement(_) => "vote",
                                        NetworkMessage::Custom(m) => &m.message_type,
                                    };
                                    
                                    // Get approximate size
                                    if let Ok(bytes) = bincode::serialize(&message) {
                                        metrics.record_message_sent(message_type, bytes.len());
                                    }
                                }
                            }
                            Some(Command::SendTo(peer_id, message)) => {
                                let start_time = Instant::now();
                                
                                Self::handle_send_to(&mut swarm, &peer_id, message.clone()).await;
                                
                                // Record metrics
                                if let Some(metrics) = &self.metrics {
                                    let elapsed = start_time.elapsed();
                                    metrics.record_message_processing_time(elapsed);
                                    
                                    // Record message size approximately
                                    let message_type = match &message {
                                        NetworkMessage::IdentityAnnouncement(_) => "identity",
                                        NetworkMessage::TransactionAnnouncement(_) => "transaction",
                                        NetworkMessage::LedgerStateUpdate(_) => "ledger",
                                        NetworkMessage::ProposalAnnouncement(_) => "proposal",
                                        NetworkMessage::VoteAnnouncement(_) => "vote",
                                        NetworkMessage::Custom(m) => &m.message_type,
                                    };
                                    
                                    // Get approximate size
                                    if let Ok(bytes) = bincode::serialize(&message) {
                                        metrics.record_message_sent(message_type, bytes.len());
                                    }
                                }
                            }
                            Some(Command::Connect(addr, response_tx)) => {
                                // Record connection attempt
                                if let Some(metrics) = &self.metrics {
                                    metrics.record_connection_attempt();
                                }
                                
                                let result = Self::handle_connect(&mut swarm, addr, response_tx).await;
                                
                                // Record connection result
                                if let Some(metrics) = &self.metrics {
                                    if result.is_ok() {
                                        metrics.record_connection_success();
                                    } else {
                                        metrics.record_connection_failure();
                                    }
                                }
                            }
                            Some(Command::Disconnect(peer_id, response_tx)) => {
                                Self::handle_disconnect(&mut swarm, &peer_id, response_tx).await;
                                
                                // Record disconnection
                                if let Some(metrics) = &self.metrics {
                                    metrics.record_peer_disconnected();
                                }
                            }
                            Some(Command::GetPeerInfo(peer_id, response_tx)) => {
                                Self::handle_get_peer_info(&peers, &peer_id, response_tx).await;
                            }
                            Some(Command::GetConnectedPeers(response_tx)) => {
                                Self::handle_get_connected_peers(&peers, response_tx).await;
                            }
                            Some(Command::RegisterHandler(message_type, handler, response_tx)) => {
                                Self::handle_register_handler(&handlers, message_type, handler, response_tx).await;
                            }
                            Some(Command::GetListenAddresses(response_tx)) => {
                                let addrs = listen_addresses.read().await.clone();
                                let _ = response_tx.send(Ok(addrs)).await;
                            }
                            Some(Command::Stop(response_tx)) => {
                                info!("Stopping P2P network task");
                                let _ = response_tx.send(Ok(())).await;
                                
                                // Reset metrics
                                if let Some(metrics) = &self.metrics {
                                    metrics.reset();
                                }
                                
                                break;
                            }
                            None => {
                                // Channel closed, exit loop
                                error!("Command channel closed unexpectedly");
                                break;
                            }
                        }
                    }
                }
            }
            
            info!("P2P network task stopped");
        });
        
        // Store the task handle
        let mut handle = self.task_handle.lock().await;
        *handle = Some(task);
        
        Ok(())
    }
    
    /// Handle swarm events
    async fn handle_swarm_event(
        event: SwarmEvent<ComposedEvent>,
        swarm: &mut swarm::Swarm<P2pBehaviour>,
        handlers: &Arc<RwLock<HashMap<String, Vec<Arc<dyn MessageHandler>>>>>,
        peers: &Arc<RwLock<HashMap<PeerId, PeerInfo>>>,
        metrics: Option<&NetworkMetrics>,
        reputation: Option<&Arc<ReputationManager>>,
        message_processor: Option<&Arc<messaging::MessageProcessor>>,
    ) {
        match event {
            SwarmEvent::NewListenAddr { address, .. } => {
                info!("Listening on {}", address);
            }
            SwarmEvent::ConnectionEstablished { peer_id, endpoint, num_established, .. } => {
                if num_established == 1 {
                    // This is a new connection
                    debug!("Connection established with peer: {}", peer_id);
                    let addr = endpoint.get_remote_address().clone();
                    Self::update_peer_connection(peers, &peer_id, true, Some(addr)).await;
                    
                    // Record connection established
                    if let Some(m) = metrics {
                        m.record_peer_connected();
                    }
                    
                    // Update reputation
                    if let Some(rep) = reputation {
                        let _ = rep.record_change(&peer_id, ReputationChange::ConnectionEstablished).await;
                    }
                }
            }
            SwarmEvent::ConnectionClosed { peer_id, cause, num_established, .. } => {
                if num_established == 0 {
                    // All connections to this peer are closed
                    debug!("Connection closed with peer: {}, cause: {:?}", peer_id, cause);
                    Self::update_peer_connection(peers, &peer_id, false, None).await;
                    
                    // Record connection closed
                    if let Some(m) = metrics {
                        m.record_peer_disconnected();
                    }
                    
                    // Update reputation based on cause
                    if let Some(rep) = reputation {
                        match cause {
                            Some(libp2p::swarm::DialError::ConnectionLimit(_)) => {
                                // Don't penalize for connection limits
                            }
                            Some(libp2p::swarm::DialError::Transport(_)) |
                            Some(libp2p::swarm::DialError::NoAddresses) => {
                                // Connection issues
                                let _ = rep.record_change(&peer_id, ReputationChange::ConnectionLost).await;
                            }
                            _ => {
                                // Other unexpected disconnects
                                let _ = rep.record_change(&peer_id, ReputationChange::ConnectionLost).await;
                            }
                        }
                    }
                }
            }
            SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
                warn!("Outgoing connection error to {:?}: {}", peer_id, error);
                
                // Record connection failure in metrics
                if let Some(m) = metrics {
                    m.record_connection_failure();
                    
                    // Record specific error type
                    let error_type = match &error {
                        libp2p::swarm::DialError::Transport(_) => "transport",
                        libp2p::swarm::DialError::LocalPeerId => "local_peer_id",
                        _ => "other",
                    };
                    
                    m.record_error(error_type);
                }
                
                // Update reputation if peer ID is available
                if let Some(peer_id) = peer_id {
                    if let Some(rep) = reputation {
                        let _ = rep.record_change(&peer_id, ReputationChange::ConnectionLost).await;
                    }
                }
            }
            SwarmEvent::Behaviour(ComposedEvent::Gossipsub(gossipsub::Event::Message { 
                propagation_source,
                message_id,
                message,
            })) => {
                // Record message received metrics
                if let Some(m) = metrics {
                    m.record_message_received("gossipsub", message.data.len());
                }
                
                debug!("Received gossip message: {} from {}", message_id, propagation_source);
                
                // Extract message type from the topic
                let topic = &message.topic;
                let message_type = match topic.as_str() {
                    TOPIC_IDENTITY => "identity.announcement",
                    TOPIC_TRANSACTIONS => "ledger.transaction",
                    TOPIC_LEDGER => "ledger.state",
                    TOPIC_GOVERNANCE => {
                        // For governance, we need to look at the message content to determine if it's a proposal or vote
                        // This is a simplification; in a real system we would have a more robust mechanism
                        if message.data.starts_with(b"proposal") {
                            "governance.proposal"
                        } else {
                            "governance.vote"
                        }
                    },
                    _ => {
                        // For unknown topics, we'll just use the topic name
                        topic.as_str()
                    }
                };
                
                // First check if using message processor
                if let Some(processor) = message_processor {
                    // Get peer info
                    let peer_info = Self::get_peer_info_from_id(peers, &propagation_source).await;
                    
                    // Deserialize the message
                    match serde_json::from_slice::<NetworkMessage>(&message.data) {
                        Ok(network_message) => {
                            // Process with priority-based processor
                            let mut net_message = network_message;
                            // Ensure correct message type from topic
                            net_message.message_type = message_type.to_string();
                            
                            if let Err(e) = processor.process_message(net_message, peer_info).await {
                                error!("Failed to process message: {}", e);
                                
                                // Record error and update reputation
                                if let Some(m) = metrics {
                                    m.record_error("message_processing");
                                }
                                
                                if let Some(rep) = reputation {
                                    let _ = rep.record_change(&propagation_source, ReputationChange::MessageFailure).await;
                                }
                            }
                        }
                        Err(e) => {
                            warn!("Failed to deserialize gossip message: {}", e);
                            
                            // Record error
                            if let Some(m) = metrics {
                                m.record_error("message_deserialization");
                            }
                            
                            // Update reputation for invalid message
                            if let Some(rep) = reputation {
                                let _ = rep.record_change(&propagation_source, ReputationChange::InvalidMessage).await;
                            }
                        }
                    }
                } else {
                    // Fall back to direct handler calling if no message processor
                    let start_time = Instant::now();
                    let mut handled_successfully = false;
                    
                    match serde_json::from_slice::<NetworkMessage>(&message.data) {
                        Ok(network_message) => {
                            // Get peer info
                            let peer_info = Self::get_peer_info_from_id(peers, &propagation_source).await;
                            
                            // Call all handlers for this message type
                            let handlers_guard = handlers.read().await;
                            if let Some(type_handlers) = handlers_guard.get(message_type) {
                                let mut success = true;
                                
                                for handler in type_handlers {
                                    if let Err(e) = handler.handle_message(&network_message, &peer_info).await {
                                        error!("Handler error: {}", e);
                                        success = false;
                                        
                                        // Update reputation for message failure
                                        if let Some(rep) = reputation {
                                            let _ = rep.record_change(&propagation_source, ReputationChange::MessageFailure).await;
                                        }
                                    }
                                }
                                
                                // Update reputation for successful message handling
                                if success {
                                    handled_successfully = true;
                                    if let Some(rep) = reputation {
                                        let _ = rep.record_change(&propagation_source, ReputationChange::MessageSuccess).await;
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            warn!("Failed to deserialize gossip message: {}", e);
                            
                            // Update reputation for invalid message
                            if let Some(rep) = reputation {
                                let _ = rep.record_change(&propagation_source, ReputationChange::InvalidMessage).await;
                            }
                        }
                    }
                    
                    // Record message processing time
                    let elapsed = start_time.elapsed();
                    if let Some(m) = metrics {
                        m.record_message_processing_time(elapsed);
                    }
                    
                    // Update reputation based on processing time
                    if let Some(rep) = reputation {
                        let _ = rep.record_response_time(&propagation_source, elapsed).await;
                        
                        // Record verification success/failure
                        if handled_successfully {
                            let _ = rep.record_change(&propagation_source, ReputationChange::VerifiedMessage).await;
                        }
                    }
                }
            }
            SwarmEvent::Behaviour(ComposedEvent::Ping(ping::Event {
                peer,
                result: Ok(rtt),
                ..
            })) => {
                // Record ping/latency metrics
                if let Some(m) = metrics {
                    m.record_peer_latency(&peer.to_string(), rtt).await;
                }
                
                // Update reputation based on ping time
                if let Some(rep) = reputation {
                    let _ = rep.record_response_time(&peer, rtt).await;
                }
                
                debug!("Ping to {} took {:?}", peer, rtt);
            }
            SwarmEvent::Behaviour(ComposedEvent::Mdns(mdns::Event::Discovered(list))) => {
                for (peer_id, addr) in list {
                    debug!("mDNS discovered peer: {} at {}", peer_id, addr);
                    
                    // Record mDNS discovery
                    if let Some(m) = metrics {
                        m.record_mdns_discovery();
                        m.record_peer_discovered();
                    }
                    
                    // Update reputation for discovery help
                    if let Some(rep) = reputation {
                        let _ = rep.record_change(&peer_id, ReputationChange::DiscoveryHelp).await;
                    }
                    
                    // Update peer info
                    Self::update_peer_connection(peers, &peer_id, false, Some(addr.clone())).await;
                    
                    // Check if the peer is banned before trying to dial
                    let should_dial = if let Some(rep) = reputation {
                        !rep.is_banned(&peer_id).await
                    } else {
                        true
                    };
                    
                    // Try to dial the peer if not banned and not connected
                    if should_dial && !swarm.is_connected(&peer_id) {
                        debug!("Dialing discovered peer {}", peer_id);
                        match swarm.dial(addr) {
                            Ok(_) => {}
                            Err(e) => warn!("Failed to dial discovered peer {}: {}", peer_id, e),
                        }
                    }
                }
            }
            SwarmEvent::Behaviour(ComposedEvent::Kad(kad::Event::RoutingUpdated {
                peer,
                addresses,
                ..
            })) => {
                debug!("Kademlia routing updated for peer: {}", peer);
                
                // Record Kademlia discovery
                if let Some(m) = metrics {
                    m.record_kad_discovery();
                    m.record_peer_discovered();
                }
                
                // Update reputation for discovery help
                if let Some(rep) = reputation {
                    let _ = rep.record_change(&peer, ReputationChange::DiscoveryHelp).await;
                }
                
                // Update peer info
                Self::update_peer_info(peers, &peer, &addresses, &[]).await;
            }
            _ => {} // Ignore other events
        }
    }
    
    /// Update peer connection status
    async fn update_peer_connection(
        peers: &Arc<RwLock<HashMap<PeerId, PeerInfo>>>, 
        peer_id: &PeerId,
        connected: bool,
        addr: Option<Multiaddr>,
    ) {
        let mut peers_guard = peers.write().await;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        let entry = peers_guard.entry(*peer_id).or_insert_with(|| {
            PeerInfo {
                peer_id: *peer_id,
                addresses: Vec::new(),
                protocols: Vec::new(),
                connected: false,
                last_seen: now,
            }
        });
        
        entry.last_seen = now;
        entry.connected = connected;
        
        if let Some(addr) = addr {
            if !entry.addresses.contains(&addr) {
                entry.addresses.push(addr);
            }
        }
    }
    
    /// Update peer info
    async fn update_peer_info(
        peers: &Arc<RwLock<HashMap<PeerId, PeerInfo>>>, 
        peer_id: &PeerId,
        addresses: &[Multiaddr],
        protocols: &[String],
    ) {
        let mut peers_guard = peers.write().await;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        let entry = peers_guard.entry(*peer_id).or_insert_with(|| {
            PeerInfo {
                peer_id: *peer_id,
                addresses: Vec::new(),
                protocols: Vec::new(),
                connected: false,
                last_seen: now,
            }
        });
        
        entry.last_seen = now;
        
        // Update addresses
        for addr in addresses {
            if !entry.addresses.contains(addr) {
                entry.addresses.push(addr.clone());
            }
        }
        
        // Update protocols
        entry.protocols = protocols.to_vec();
    }
    
    /// Get peer info from ID
    async fn get_peer_info_from_id(
        peers: &Arc<RwLock<HashMap<PeerId, PeerInfo>>>,
        peer_id: &PeerId,
    ) -> PeerInfo {
        let peers_guard = peers.read().await;
        if let Some(info) = peers_guard.get(peer_id) {
            info.clone()
        } else {
            // Create a minimal PeerInfo if we don't have it
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            
            PeerInfo {
                peer_id: *peer_id,
                addresses: Vec::new(),
                protocols: Vec::new(),
                connected: true, // Assume connected since we're receiving messages
                last_seen: now,
            }
        }
    }
    
    /// Handle broadcast command
    async fn handle_broadcast(
        swarm: &mut swarm::Swarm<P2pBehaviour>,
        message: NetworkMessage,
    ) {
        // Serialize the message
        let data = match serde_json::to_vec(&message) {
            Ok(data) => data,
            Err(e) => {
                error!("Failed to serialize broadcast message: {}", e);
                return;
            }
        };
        
        // Choose the appropriate topic based on message type
        let topic_name = match &message {
            NetworkMessage::IdentityAnnouncement(_) => TOPIC_IDENTITY,
            NetworkMessage::TransactionAnnouncement(_) => TOPIC_TRANSACTIONS,
            NetworkMessage::LedgerStateUpdate(_) => TOPIC_LEDGER,
            NetworkMessage::ProposalAnnouncement(_) | NetworkMessage::VoteAnnouncement(_) => TOPIC_GOVERNANCE,
            NetworkMessage::Custom(custom) => {
                // For custom messages, we use the governance topic
                TOPIC_GOVERNANCE
            }
        };
        
        let topic = IdentTopic::new(topic_name);
        
        // Publish the message
        if let Err(e) = swarm.behaviour_mut().gossipsub.publish(topic, data) {
            error!("Failed to publish message: {}", e);
        }
    }
    
    /// Handle send message command
    async fn handle_send_to(
        swarm: &mut swarm::Swarm<P2pBehaviour>,
        peer_id: &PeerId,
        message: NetworkMessage,
    ) {
        // For direct messages, we use gossipsub for now
        // In a more complete implementation, we would use request-response or a direct messaging protocol
        Self::handle_broadcast(swarm, message).await;
    }
    
    /// Handle connect command
    async fn handle_connect(
        swarm: &mut swarm::Swarm<P2pBehaviour>,
        addr: Multiaddr,
        response_tx: mpsc::Sender<NetworkResult<PeerId>>,
    ) -> NetworkResult<PeerId> {
        // Try to dial the address
        match swarm.dial(addr.clone()) {
            Ok(_) => {
                info!("Dialing {}", addr);
                
                // In a real implementation, we would wait for the connection to be established
                // and then return the peer ID. For now, we'll just return success with a placeholder.
                // We'll assume this is using multiaddr with a peer ID component for now
                if let Some(peer_id) = addr.iter().find_map(|p| {
                    if let libp2p::multiaddr::Protocol::P2p(hash) = p {
                        Some(PeerId::from_multihash(hash).ok()?)
                    } else {
                        None
                    }
                }) {
                    let _ = response_tx.send(Ok(peer_id)).await;
                } else {
                    // If we can't extract a peer ID, return an error
                    let err = NetworkError::InternalError(
                        "Dialing successful, but could not determine peer ID".to_string()
                    );
                    let _ = response_tx.send(Err(err)).await;
                }
            }
            Err(e) => {
                let err = NetworkError::InternalError(format!("Failed to dial {}: {}", addr, e));
                let _ = response_tx.send(Err(err)).await;
            }
        }
    }
    
    /// Handle disconnect command
    async fn handle_disconnect(
        swarm: &mut swarm::Swarm<P2pBehaviour>,
        peer_id: &PeerId,
        response_tx: mpsc::Sender<NetworkResult<()>>,
    ) {
        // Try to disconnect
        if swarm.disconnect_peer_id(*peer_id).is_ok() {
            info!("Disconnected from {}", peer_id);
            let _ = response_tx.send(Ok(())).await;
        } else {
            warn!("Failed to disconnect from {}", peer_id);
            let err = NetworkError::PeerNotFound(peer_id.to_string());
            let _ = response_tx.send(Err(err)).await;
        }
    }
    
    /// Handle get peer info command
    async fn handle_get_peer_info(
        peers: &Arc<RwLock<HashMap<PeerId, PeerInfo>>>,
        peer_id: &PeerId,
        response_tx: mpsc::Sender<NetworkResult<PeerInfo>>,
    ) {
        let peers_guard = peers.read().await;
        if let Some(info) = peers_guard.get(peer_id) {
            let _ = response_tx.send(Ok(info.clone())).await;
        } else {
            let err = NetworkError::PeerNotFound(peer_id.to_string());
            let _ = response_tx.send(Err(err)).await;
        }
    }
    
    /// Handle get connected peers command
    async fn handle_get_connected_peers(
        peers: &Arc<RwLock<HashMap<PeerId, PeerInfo>>>,
        response_tx: mpsc::Sender<NetworkResult<Vec<PeerInfo>>>,
    ) {
        let peers_guard = peers.read().await;
        let connected_peers = peers_guard.values()
            .filter(|p| p.connected)
            .cloned()
            .collect();
        let _ = response_tx.send(Ok(connected_peers)).await;
    }
    
    /// Handle register handler command
    async fn handle_register_handler(
        handlers: &Arc<RwLock<HashMap<String, Vec<Arc<dyn MessageHandler>>>>>,
        message_type: String,
        handler: Arc<dyn MessageHandler>,
        response_tx: mpsc::Sender<NetworkResult<()>>,
    ) {
        let mut handlers_guard = handlers.write().await;
        
        let type_handlers = handlers_guard.entry(message_type.clone()).or_insert_with(Vec::new);
        type_handlers.push(handler);
        
        debug!("Registered handler for message type: {}", message_type);
        let _ = response_tx.send(Ok(())).await;
    }
    
    /// Check if a peer is allowed to connect
    pub async fn is_peer_allowed(&self, peer_id: &PeerId) -> bool {
        // If reputation system is enabled, check if the peer is banned
        if let Some(rep) = &self.reputation {
            return !rep.is_banned(peer_id).await;
        }
        
        // Otherwise, all peers are allowed
        true
    }
    
    /// Get the reputation manager
    pub fn reputation_manager(&self) -> Option<Arc<ReputationManager>> {
        self.reputation.clone()
    }
    
    /// Update peer reputation
    pub async fn update_reputation(&self, peer_id: &PeerId, change: ReputationChange) -> NetworkResult<()> {
        if let Some(rep) = &self.reputation {
            rep.record_change(peer_id, change).await?;
        }
        
        Ok(())
    }
    
    /// Ban a peer
    pub async fn ban_peer(&self, peer_id: &PeerId) -> NetworkResult<()> {
        if let Some(rep) = &self.reputation {
            rep.ban_peer(peer_id).await?;
            
            // Also disconnect from the peer
            self.disconnect(peer_id).await?;
        }
        
        Ok(())
    }
    
    /// Unban a peer
    pub async fn unban_peer(&self, peer_id: &PeerId) -> NetworkResult<()> {
        if let Some(rep) = &self.reputation {
            rep.unban_peer(peer_id).await?;
        }
        
        Ok(())
    }
    
    /// Get message queue statistics
    pub async fn get_message_queue_stats(&self) -> NetworkResult<(usize, Option<i32>, Option<i32>)> {
        if let Some(processor) = &self.message_processor {
            let stats = processor.queue_stats().await;
            
            // Record metrics if available
            if let Some(metrics) = &self.metrics {
                metrics.record_queue_stats(stats.0, stats.1, stats.2);
            }
            
            Ok(stats)
        } else {
            // Return zeros if message processor isn't enabled
            Ok((0, None, None))
        }
    }
    
    /// Connect to a peer using the best available method (direct or relay)
    pub async fn smart_connect(&self, peer_id: &PeerId) -> NetworkResult<()> {
        // First try direct connection if we have addresses
        let connected = {
            let peers = self.peers.read().await;
            if let Some(peer_info) = peers.get(peer_id) {
                if !peer_info.addresses.is_empty() {
                    // Try direct connection first
                    for addr in &peer_info.addresses {
                        let result = self.connect(addr).await;
                        if result.is_ok() {
                            return Ok(());
                        }
                    }
                }
            }
            false
        };
        
        if !connected && self.config.enable_circuit_relay {
            // Try connecting via relay if direct connection failed
            if let Some(relay_manager) = &self.circuit_relay {
                match relay_manager.connect_via_relay(peer_id).await {
                    Ok(relay_addr) => {
                        // Connect via the relay address
                        self.connect(&relay_addr).await?;
                        return Ok(());
                    }
                    Err(e) => {
                        warn!("Failed to connect via relay to {}: {}", peer_id, e);
                    }
                }
            }
        }
        
        Err(NetworkError::ConnectionFailed)
    }
    
    /// Check if a peer is connected via relay
    pub async fn is_relay_connection(&self, peer_id: &PeerId) -> bool {
        if let Some(relay_manager) = &self.circuit_relay {
            relay_manager.is_relayed_connection(peer_id).await
        } else {
            false
        }
    }
    
    /// Get the relay used for a connection
    pub async fn get_relay_for_connection(&self, peer_id: &PeerId) -> Option<PeerId> {
        if let Some(relay_manager) = &self.circuit_relay {
            relay_manager.get_relay_for_connection(peer_id).await
        } else {
            None
        }
    }
    
    /// Get a list of known relay servers
    pub async fn get_relay_servers(&self) -> Vec<String> {
        if let Some(relay_manager) = &self.circuit_relay {
            let servers = relay_manager.get_relay_servers().await;
            servers.into_iter().map(|server| server.peer_id.to_string()).collect()
        } else {
            Vec::new()
        }
    }
    
    /// Add a relay server
    pub async fn add_relay_server(&self, addr: &Multiaddr) -> NetworkResult<()> {
        if let Some(relay_manager) = &self.circuit_relay {
            if let Some(peer_id) = extract_peer_id(addr) {
                relay_manager.add_relay_server(peer_id, vec![addr.clone()]).await?;
                Ok(())
            } else {
                Err(NetworkError::InvalidRelayAddress)
            }
        } else {
            Err(NetworkError::ServiceNotEnabled("Circuit relay is not enabled".to_string()))
        }
    }
}

// Extract peer ID from a multiaddress
fn extract_peer_id(addr: &Multiaddr) -> Option<PeerId> {
    addr.iter().find_map(|p| match p {
        libp2p::multiaddr::Protocol::P2p(hash) => Some(PeerId::from_multihash(hash).ok()?),
        _ => None,
    })
}

#[async_trait]
impl NetworkService for P2pNetwork {
    async fn start(&self) -> NetworkResult<()> {
        // The network task is already started in new()
        Ok(())
    }
    
    async fn stop(&self) -> NetworkResult<()> {
        let (tx, rx) = mpsc::channel(1);

        if let Err(e) = self.command_tx.send(Command::Stop(tx)).await {
            return Err(NetworkError::ServiceError(format!("Failed to send stop command: {}", e)));
        }

        // Wait for the response
        match rx.await {
            Ok(result) => {
                // Also stop the message processor if it exists
                if let Some(processor) = &self.message_processor {
                    if let Err(e) = processor.stop().await {
                        warn!("Error stopping message processor: {}", e);
                    }
                }
                
                // Also stop the reputation manager if it exists
                if let Some(rep) = &self.reputation {
                    if let Err(e) = rep.stop().await {
                        warn!("Error stopping reputation manager: {}", e);
                    }
                }
                
                result
            },
            Err(e) => Err(NetworkError::ServiceError(format!("Failed to receive stop response: {}", e))),
        }
    }
    
    async fn broadcast(&self, message: NetworkMessage) -> NetworkResult<()> {
        self.command_tx.send(Command::Broadcast(message)).await
            .map_err(|_| NetworkError::InternalError("Failed to send broadcast command".to_string()))?;
        Ok(())
    }
    
    async fn send_to(&self, peer_id: &PeerId, message: NetworkMessage) -> NetworkResult<()> {
        self.command_tx.send(Command::SendTo(*peer_id, message)).await
            .map_err(|_| NetworkError::InternalError("Failed to send message command".to_string()))?;
        Ok(())
    }
    
    async fn connect(&self, addr: &Multiaddr) -> NetworkResult<PeerId> {
        let (tx, rx) = mpsc::channel(1);
        self.command_tx.send(Command::Connect(addr.clone(), tx)).await
            .map_err(|_| NetworkError::InternalError("Failed to send connect command".to_string()))?;
        
        // Wait for the connect command to complete
        rx.await
            .map_err(|_| NetworkError::InternalError("Failed to receive connect response".to_string()))?
    }
    
    async fn disconnect(&self, peer_id: &PeerId) -> NetworkResult<()> {
        let (tx, rx) = mpsc::channel(1);
        self.command_tx.send(Command::Disconnect(*peer_id, tx)).await
            .map_err(|_| NetworkError::InternalError("Failed to send disconnect command".to_string()))?;
        
        // Wait for the disconnect command to complete
        rx.await
            .map_err(|_| NetworkError::InternalError("Failed to receive disconnect response".to_string()))?
    }
    
    async fn get_peer_info(&self, peer_id: &PeerId) -> NetworkResult<PeerInfo> {
        let (tx, rx) = mpsc::channel(1);
        self.command_tx.send(Command::GetPeerInfo(*peer_id, tx)).await
            .map_err(|_| NetworkError::InternalError("Failed to send get_peer_info command".to_string()))?;
        
        // Wait for the get_peer_info command to complete
        rx.await
            .map_err(|_| NetworkError::InternalError("Failed to receive get_peer_info response".to_string()))?
    }
    
    async fn get_connected_peers(&self) -> NetworkResult<Vec<PeerInfo>> {
        let (tx, rx) = mpsc::channel(1);
        self.command_tx.send(Command::GetConnectedPeers(tx)).await
            .map_err(|_| NetworkError::InternalError("Failed to send get_connected_peers command".to_string()))?;
        
        // Wait for the get_connected_peers command to complete
        rx.await
            .map_err(|_| NetworkError::InternalError("Failed to receive get_connected_peers response".to_string()))?
    }
    
    async fn register_message_handler(&self, message_type: &str, handler: Arc<dyn MessageHandler>) -> NetworkResult<()> {
        let (tx, rx) = mpsc::channel(1);
        self.command_tx.send(Command::RegisterHandler(message_type.to_string(), handler, tx)).await
            .map_err(|_| NetworkError::InternalError("Failed to send register_handler command".to_string()))?;
        
        // Wait for the register_handler command to complete
        rx.await
            .map_err(|_| NetworkError::InternalError("Failed to receive register_handler response".to_string()))?
    }
} 