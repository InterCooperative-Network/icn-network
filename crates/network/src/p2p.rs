//! P2P network implementation using libp2p
//!
//! This module provides the core implementation of the P2P network
//! functionality for the ICN.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use futures::prelude::*;
use libp2p::{
    core::{muxing::StreamMuxerBox, transport::OrTransport, upgrade},
    gossipsub::{self, IdentTopic, MessageAuthenticity, MessageId, ValidationMode},
    identify, kad, mdns, noise, ping, request_response,
    swarm::{self, NetworkBehaviour, SwarmBuilder, SwarmEvent},
    tcp, yamux, Multiaddr, PeerId, Transport,
    identity::Keypair,
};
use tokio::sync::{mpsc, RwLock, Mutex};
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

use icn_core::{
    crypto::{NodeId, Signature},
    storage::Storage,
    utils::timestamp_secs,
};

use icn_identity::IdentityProvider;

use crate::{
    MessageHandler, NetworkError, NetworkMessage, NetworkResult, NetworkService,
    PeerInfo,
};

// Topic names for gossipsub
const TOPIC_IDENTITY: &str = "icn/identity/v1";
const TOPIC_TRANSACTIONS: &str = "icn/transactions/v1";
const TOPIC_LEDGER: &str = "icn/ledger/v1";
const TOPIC_GOVERNANCE: &str = "icn/governance/v1";

/// Configuration for the P2P network
#[derive(Clone, Debug)]
pub struct P2pConfig {
    /// Local listening addresses
    pub listen_addresses: Vec<Multiaddr>,
    /// Bootstrap peers
    pub bootstrap_peers: Vec<Multiaddr>,
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
            keep_alive: Duration::from_secs(20),
            peer_store_path: None,
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
    SendMessage(PeerId, NetworkMessage),
    /// Connect to a peer
    Connect(Multiaddr, mpsc::Sender<NetworkResult<PeerId>>),
    /// Disconnect from a peer
    Disconnect(PeerId, mpsc::Sender<NetworkResult<()>>),
    /// Get information about a peer
    GetPeerInfo(PeerId, mpsc::Sender<NetworkResult<Option<PeerInfo>>>),
    /// Get a list of connected peers
    GetConnectedPeers(mpsc::Sender<NetworkResult<Vec<PeerInfo>>>),
    /// Stop the network service
    Stop(mpsc::Sender<NetworkResult<()>>),
}

/// The main P2P network service implementation
pub struct P2pNetwork {
    /// Identity provider for authentication
    identity_provider: Arc<dyn IdentityProvider>,
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
    handlers: Arc<RwLock<Vec<Arc<dyn MessageHandler>>>>,
    /// Known peers
    peers: Arc<RwLock<HashMap<PeerId, PeerInfo>>>,
}

impl P2pNetwork {
    /// Create a new P2P network
    pub async fn new(
        identity_provider: Arc<dyn IdentityProvider>,
        storage: Arc<dyn Storage>,
        config: P2pConfig,
    ) -> NetworkResult<Self> {
        // Generate or load key pair
        let key_pair = Self::load_or_create_keypair(storage.clone()).await?;
        let local_peer_id = PeerId::from(key_pair.public());
        
        // Create the command channel
        let (command_tx, command_rx) = mpsc::channel(100);
        
        let network = Self {
            identity_provider,
            storage,
            key_pair,
            local_peer_id,
            config,
            command_tx,
            task_handle: Arc::new(Mutex::new(None)),
            handlers: Arc::new(RwLock::new(Vec::new())),
            peers: Arc::new(RwLock::new(HashMap::new())),
        };
        
        // Start the background task
        network.start_background_task(command_rx).await?;
        
        Ok(network)
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
        // Set up an encrypted TCP transport
        let noise_keys = noise::Keypair::<noise::X25519Spec>::new()
            .into_authentic(key_pair)
            .map_err(|e| NetworkError::Libp2pError(format!("Signing libp2p-noise static DH keypair failed: {}", e)))?;
        
        let transport = tcp::async_io::Transport::new(tcp::Config::default())
            .upgrade(upgrade::Version::V1)
            .authenticate(noise::NoiseConfig::xx(noise_keys).into_authenticated())
            .multiplex(yamux::YamuxConfig::default())
            .timeout(Duration::from_secs(20))
            .boxed();
        
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
        
        let swarm = SwarmBuilder::with_async_std_executor(
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
            match swarm.dial(addr.clone()) {
                Ok(_) => info!("Dialing bootstrap peer {}", addr),
                Err(e) => warn!("Failed to dial bootstrap peer {}: {}", addr, e),
            }
        }
        
        // Clone needed components for the task
        let handlers = self.handlers.clone();
        let peers = self.peers.clone();
        let peer_id = self.local_peer_id;
        let identity_provider = self.identity_provider.clone();
        
        // Start the network task
        let task = tokio::spawn(async move {
            info!("P2P network task started, peer ID: {}", peer_id);
            
            loop {
                tokio::select! {
                    event = swarm.select_next_some() => {
                        Self::handle_swarm_event(
                            event, 
                            &mut swarm, 
                            &handlers,
                            &peers,
                            &identity_provider,
                        ).await;
                    }
                    cmd = command_rx.recv() => {
                        match cmd {
                            Some(Command::Broadcast(message)) => {
                                Self::handle_broadcast(&mut swarm, message).await;
                            }
                            Some(Command::SendMessage(peer_id, message)) => {
                                Self::handle_send_message(&mut swarm, &peer_id, message).await;
                            }
                            Some(Command::Connect(addr, response_tx)) => {
                                Self::handle_connect(&mut swarm, addr, response_tx).await;
                            }
                            Some(Command::Disconnect(peer_id, response_tx)) => {
                                Self::handle_disconnect(&mut swarm, &peer_id, response_tx).await;
                            }
                            Some(Command::GetPeerInfo(peer_id, response_tx)) => {
                                Self::handle_get_peer_info(&peers, &peer_id, response_tx).await;
                            }
                            Some(Command::GetConnectedPeers(response_tx)) => {
                                Self::handle_get_connected_peers(&peers, response_tx).await;
                            }
                            Some(Command::Stop(response_tx)) => {
                                info!("Stopping P2P network task");
                                let _ = response_tx.send(Ok(())).await;
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
        handlers: &Arc<RwLock<Vec<Arc<dyn MessageHandler>>>>,
        peers: &Arc<RwLock<HashMap<PeerId, PeerInfo>>>,
        identity_provider: &Arc<dyn IdentityProvider>,
    ) {
        match event {
            SwarmEvent::NewListenAddr { address, .. } => {
                info!("Listening on {}", address);
            }
            SwarmEvent::ConnectionEstablished { peer_id, endpoint, .. } => {
                info!("Connection established with {} via {}", peer_id, endpoint.get_remote_address());
                
                // Update peer info
                Self::update_peer_connection(peers, &peer_id, true, Some(endpoint.get_remote_address().clone())).await;
            }
            SwarmEvent::ConnectionClosed { peer_id, cause, .. } => {
                info!("Connection closed with {}: {:?}", peer_id, cause);
                
                // Update peer info
                Self::update_peer_connection(peers, &peer_id, false, None).await;
            }
            SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
                warn!("Outgoing connection error: {:?} for peer {:?}", error, peer_id);
            }
            SwarmEvent::Behaviour(ComposedEvent::Gossipsub(gossipsub::Event::Message { 
                propagation_source,
                message_id,
                message,
            })) => {
                debug!(
                    "Received gossip message: {} from {}",
                    message_id, propagation_source
                );
                
                match serde_json::from_slice::<NetworkMessage>(&message.data) {
                    Ok(network_message) => {
                        // Call all handlers
                        let handlers_guard = handlers.read().await;
                        for handler in handlers_guard.iter() {
                            if let Err(e) = handler.handle_message(&propagation_source, network_message.clone()).await {
                                error!("Handler error: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Failed to deserialize gossip message: {}", e);
                    }
                }
            }
            SwarmEvent::Behaviour(ComposedEvent::Mdns(mdns::Event::Discovered(list))) => {
                for (peer_id, addr) in list {
                    info!("mDNS discovered peer {} at {}", peer_id, addr);
                    
                    // Update peer info
                    Self::update_peer_connection(peers, &peer_id, false, Some(addr)).await;
                    
                    // Try to dial the peer if not connected
                    if !swarm.is_connected(&peer_id) {
                        debug!("Dialing discovered peer {}", peer_id);
                        match swarm.dial(addr) {
                            Ok(_) => {}
                            Err(e) => warn!("Failed to dial discovered peer {}: {}", peer_id, e),
                        }
                    }
                }
            }
            SwarmEvent::Behaviour(ComposedEvent::Identify(identify::Event::Received { 
                peer_id,
                info: identify::Info { protocol_version, agent_version, listen_addrs, .. },
            })) => {
                info!(
                    "Identified peer {} as {} ({}), listening on: {:?}",
                    peer_id, agent_version, protocol_version, listen_addrs
                );
                
                // Update peer info with all addresses
                for addr in listen_addrs {
                    Self::update_peer_info(peers, &peer_id, None, Some(addr)).await;
                }
                
                // Add the addresses to the Kademlia routing table
                for addr in listen_addrs {
                    swarm.behaviour_mut().kad.add_address(&peer_id, addr);
                }
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
        let now = timestamp_secs();
        
        let entry = peers_guard.entry(*peer_id).or_insert_with(|| {
            PeerInfo {
                peer_id: *peer_id,
                node_id: None,
                addresses: HashSet::new(),
                first_seen: now,
                last_seen: now,
                connected: false,
            }
        });
        
        entry.last_seen = now;
        entry.connected = connected;
        
        if let Some(addr) = addr {
            entry.addresses.insert(addr);
        }
    }
    
    /// Update peer info
    async fn update_peer_info(
        peers: &Arc<RwLock<HashMap<PeerId, PeerInfo>>>, 
        peer_id: &PeerId,
        node_id: Option<NodeId>,
        addr: Option<Multiaddr>,
    ) {
        let mut peers_guard = peers.write().await;
        let now = timestamp_secs();
        
        let entry = peers_guard.entry(*peer_id).or_insert_with(|| {
            PeerInfo {
                peer_id: *peer_id,
                node_id: None,
                addresses: HashSet::new(),
                first_seen: now,
                last_seen: now,
                connected: false,
            }
        });
        
        entry.last_seen = now;
        
        if let Some(node_id) = node_id {
            entry.node_id = Some(node_id);
        }
        
        if let Some(addr) = addr {
            entry.addresses.insert(addr);
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
            NetworkMessage::Custom { message_type, .. } => {
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
    async fn handle_send_message(
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
    ) {
        // Try to dial the address
        match swarm.dial(addr.clone()) {
            Ok(_) => {
                info!("Dialing {}", addr);
                
                // We don't immediately know the peer ID
                // In a real implementation, we would wait for the connection to be established
                // and then return the peer ID
                // For now, we just return an error
                let _ = response_tx.send(Err(NetworkError::ConnectionFailed(
                    "Dialing successful, but peer ID not yet known".to_string()
                ))).await;
            }
            Err(e) => {
                let err = NetworkError::ConnectionFailed(format!("Failed to dial {}: {}", addr, e));
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
            let err = NetworkError::PeerNotFound(*peer_id);
            let _ = response_tx.send(Err(err)).await;
        }
    }
    
    /// Handle get peer info command
    async fn handle_get_peer_info(
        peers: &Arc<RwLock<HashMap<PeerId, PeerInfo>>>,
        peer_id: &PeerId,
        response_tx: mpsc::Sender<NetworkResult<Option<PeerInfo>>>,
    ) {
        let peers_guard = peers.read().await;
        let result = peers_guard.get(peer_id).cloned();
        let _ = response_tx.send(Ok(result)).await;
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
}

#[async_trait]
impl NetworkService for P2pNetwork {
    async fn start(&self) -> NetworkResult<()> {
        // The network task is already started in new()
        Ok(())
    }
    
    async fn stop(&self) -> NetworkResult<()> {
        let (tx, rx) = mpsc::channel(1);
        self.command_tx.send(Command::Stop(tx)).await
            .map_err(|_| NetworkError::MessageHandlingError("Failed to send stop command".to_string()))?;
        
        // Wait for the stop command to complete
        rx.await
            .map_err(|_| NetworkError::MessageHandlingError("Failed to receive stop response".to_string()))?
    }
    
    async fn broadcast(&self, message: NetworkMessage) -> NetworkResult<()> {
        self.command_tx.send(Command::Broadcast(message)).await
            .map_err(|_| NetworkError::MessageHandlingError("Failed to send broadcast command".to_string()))?;
        Ok(())
    }
    
    async fn send_message(&self, peer_id: &PeerId, message: NetworkMessage) -> NetworkResult<()> {
        self.command_tx.send(Command::SendMessage(*peer_id, message)).await
            .map_err(|_| NetworkError::MessageHandlingError("Failed to send message command".to_string()))?;
        Ok(())
    }
    
    async fn connect(&self, addr: &Multiaddr) -> NetworkResult<PeerId> {
        let (tx, rx) = mpsc::channel(1);
        self.command_tx.send(Command::Connect(addr.clone(), tx)).await
            .map_err(|_| NetworkError::MessageHandlingError("Failed to send connect command".to_string()))?;
        
        // Wait for the connect command to complete
        rx.await
            .map_err(|_| NetworkError::MessageHandlingError("Failed to receive connect response".to_string()))?
    }
    
    async fn disconnect(&self, peer_id: &PeerId) -> NetworkResult<()> {
        let (tx, rx) = mpsc::channel(1);
        self.command_tx.send(Command::Disconnect(*peer_id, tx)).await
            .map_err(|_| NetworkError::MessageHandlingError("Failed to send disconnect command".to_string()))?;
        
        // Wait for the disconnect command to complete
        rx.await
            .map_err(|_| NetworkError::MessageHandlingError("Failed to receive disconnect response".to_string()))?
    }
    
    async fn get_peer_info(&self, peer_id: &PeerId) -> NetworkResult<Option<PeerInfo>> {
        let (tx, rx) = mpsc::channel(1);
        self.command_tx.send(Command::GetPeerInfo(*peer_id, tx)).await
            .map_err(|_| NetworkError::MessageHandlingError("Failed to send get_peer_info command".to_string()))?;
        
        // Wait for the get_peer_info command to complete
        rx.await
            .map_err(|_| NetworkError::MessageHandlingError("Failed to receive get_peer_info response".to_string()))?
    }
    
    async fn get_connected_peers(&self) -> NetworkResult<Vec<PeerInfo>> {
        let (tx, rx) = mpsc::channel(1);
        self.command_tx.send(Command::GetConnectedPeers(tx)).await
            .map_err(|_| NetworkError::MessageHandlingError("Failed to send get_connected_peers command".to_string()))?;
        
        // Wait for the get_connected_peers command to complete
        rx.await
            .map_err(|_| NetworkError::MessageHandlingError("Failed to receive get_connected_peers response".to_string()))?
    }
    
    fn register_handler(&self, handler: Arc<dyn MessageHandler>) {
        tokio::spawn({
            let handlers = self.handlers.clone();
            async move {
                let mut handlers_guard = handlers.write().await;
                handlers_guard.push(handler);
            }
        });
    }
} 