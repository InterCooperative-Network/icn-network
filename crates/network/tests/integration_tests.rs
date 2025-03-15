use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use futures::future::join_all;
use tokio::sync::Mutex;
use tokio::time::sleep;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use icn_core::storage::mock_storage::MockStorage;
use icn_network::{
    DefaultMessageHandler, DiscoveryConfig, DiscoveryManager, MessageProcessor, NetworkMessage, 
    NetworkService, P2pConfig, P2pNetwork, PeerInfo, Synchronizer, SyncConfig, TransactionAnnouncement,
};

// Initialize logging for tests
fn init_logging() {
    let _ = tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "info,icn_network=debug".into()))
        .with(tracing_subscriber::fmt::layer())
        .try_init();
}

// Create a test network with a specific configuration
async fn create_test_network(
    port: u16,
    name: &str,
    enable_mdns: bool,
    bootstrap_peers: Vec<String>,
) -> Arc<P2pNetwork> {
    let storage = Arc::new(MockStorage::new());
    
    let mut config = P2pConfig::default();
    config.listen_addresses = vec![format!("/ip4/127.0.0.1/tcp/{}", port).parse().unwrap()];
    config.enable_mdns = enable_mdns;
    config.bootstrap_peers = bootstrap_peers;
    
    Arc::new(P2pNetwork::new(storage, config).await.unwrap())
}

#[tokio::test]
async fn test_network_end_to_end() {
    init_logging();
    
    // Create several test networks
    let network1 = create_test_network(9001, "node1", false, vec![]).await;
    let network2 = create_test_network(9002, "node2", false, vec![]).await;
    let network3 = create_test_network(9003, "node3", false, vec![]).await;
    
    // Start the networks
    network1.start().await.unwrap();
    network2.start().await.unwrap();
    network3.start().await.unwrap();
    
    // Wait for networks to initialize
    sleep(Duration::from_millis(100)).await;
    
    // Get the addresses of the networks
    let network1_peer_id = network1.local_peer_id();
    let network1_addr = network1.listen_addresses().await.unwrap()[0].clone();
    let network1_full_addr = format!("{}/p2p/{}", network1_addr, network1_peer_id);
    
    // Connect nodes in a chain: 1 <- 2 <- 3
    network2.connect(&network1_full_addr.parse().unwrap()).await.unwrap();
    
    let network2_peer_id = network2.local_peer_id();
    let network2_addr = network2.listen_addresses().await.unwrap()[0].clone();
    let network2_full_addr = format!("{}/p2p/{}", network2_addr, network2_peer_id);
    
    network3.connect(&network2_full_addr.parse().unwrap()).await.unwrap();
    
    // Wait for connections to establish
    sleep(Duration::from_millis(200)).await;
    
    // Setup message counting for each node
    let received_messages1 = Arc::new(Mutex::new(HashMap::new()));
    let received_messages2 = Arc::new(Mutex::new(HashMap::new()));
    let received_messages3 = Arc::new(Mutex::new(HashMap::new()));
    
    let received_messages1_clone = received_messages1.clone();
    let received_messages2_clone = received_messages2.clone();
    let received_messages3_clone = received_messages3.clone();
    
    // Create message handlers for each network
    let handler1 = Arc::new(DefaultMessageHandler::new(
        1,
        "Node1Handler".to_string(),
        move |message, _| {
            if let NetworkMessage::TransactionAnnouncement(tx) = message {
                let mut messages = received_messages1_clone.blocking_lock();
                *messages.entry(tx.transaction_id.clone()).or_insert(0) += 1;
            }
            Ok(())
        }
    ));
    
    let handler2 = Arc::new(DefaultMessageHandler::new(
        1,
        "Node2Handler".to_string(),
        move |message, _| {
            if let NetworkMessage::TransactionAnnouncement(tx) = message {
                let mut messages = received_messages2_clone.blocking_lock();
                *messages.entry(tx.transaction_id.clone()).or_insert(0) += 1;
            }
            Ok(())
        }
    ));
    
    let handler3 = Arc::new(DefaultMessageHandler::new(
        1,
        "Node3Handler".to_string(),
        move |message, _| {
            if let NetworkMessage::TransactionAnnouncement(tx) = message {
                let mut messages = received_messages3_clone.blocking_lock();
                *messages.entry(tx.transaction_id.clone()).or_insert(0) += 1;
            }
            Ok(())
        }
    ));
    
    // Register handlers
    network1.register_message_handler("ledger.transaction", handler1).await.unwrap();
    network2.register_message_handler("ledger.transaction", handler2).await.unwrap();
    network3.register_message_handler("ledger.transaction", handler3).await.unwrap();
    
    // Create and broadcast test messages
    let tx1 = TransactionAnnouncement {
        transaction_id: "tx1".to_string(),
        transaction_type: "test".to_string(),
        timestamp: 1234567890,
        sender: "node1".to_string(),
        data_hash: "hash1".to_string(),
    };
    
    let tx2 = TransactionAnnouncement {
        transaction_id: "tx2".to_string(),
        transaction_type: "test".to_string(),
        timestamp: 1234567891,
        sender: "node2".to_string(),
        data_hash: "hash2".to_string(),
    };
    
    let tx3 = TransactionAnnouncement {
        transaction_id: "tx3".to_string(),
        transaction_type: "test".to_string(),
        timestamp: 1234567892,
        sender: "node3".to_string(),
        data_hash: "hash3".to_string(),
    };
    
    // Broadcast messages from each node
    network1.broadcast(NetworkMessage::TransactionAnnouncement(tx1)).await.unwrap();
    sleep(Duration::from_millis(100)).await;
    
    network2.broadcast(NetworkMessage::TransactionAnnouncement(tx2)).await.unwrap();
    sleep(Duration::from_millis(100)).await;
    
    network3.broadcast(NetworkMessage::TransactionAnnouncement(tx3)).await.unwrap();
    
    // Wait for message propagation
    sleep(Duration::from_secs(1)).await;
    
    // Check that all messages were received by all nodes
    let messages1 = received_messages1.lock().await;
    let messages2 = received_messages2.lock().await;
    let messages3 = received_messages3.lock().await;
    
    // Node 1 should receive messages from node 2 and node 3
    assert!(messages1.contains_key("tx2"), "Node 1 did not receive tx2");
    assert!(messages1.contains_key("tx3"), "Node 1 did not receive tx3");
    
    // Node 2 should receive messages from node 1 and node 3
    assert!(messages2.contains_key("tx1"), "Node 2 did not receive tx1");
    assert!(messages2.contains_key("tx3"), "Node 2 did not receive tx3");
    
    // Node 3 should receive messages from node 1 and node 2
    assert!(messages3.contains_key("tx1"), "Node 3 did not receive tx1");
    assert!(messages3.contains_key("tx2"), "Node 3 did not receive tx2");
    
    // Test discovery manager
    let discovery_config = DiscoveryConfig::default();
    let discovery_manager = DiscoveryManager::new(
        network1.clone() as Arc<dyn NetworkService>,
        discovery_config,
    );
    
    // Start discovery
    discovery_manager.start().await.unwrap();
    
    // Test synchronizer
    let sync_config = SyncConfig::default();
    let storage = Arc::new(MockStorage::new());
    let synchronizer = Synchronizer::new(
        storage,
        network2.clone() as Arc<dyn NetworkService>,
        sync_config,
    );
    
    // Start synchronization
    synchronizer.start().await.unwrap();
    
    // Force a sync
    synchronizer.sync_with_peer(&network3.local_peer_id()).await.unwrap();
    
    // Stop networks
    let stop_futures = vec![
        network1.stop(),
        network2.stop(),
        network3.stop(),
        discovery_manager.stop(),
        synchronizer.stop(),
    ];
    
    join_all(stop_futures).await;
}

#[tokio::test]
async fn test_large_message_broadcast() {
    init_logging();
    
    // Create two test networks
    let network1 = create_test_network(9101, "large1", false, vec![]).await;
    let network2 = create_test_network(9102, "large2", false, vec![]).await;
    
    // Start the networks
    network1.start().await.unwrap();
    network2.start().await.unwrap();
    
    // Wait for networks to initialize
    sleep(Duration::from_millis(100)).await;
    
    // Connect network2 to network1
    let network1_peer_id = network1.local_peer_id();
    let network1_addr = network1.listen_addresses().await.unwrap()[0].clone();
    let network1_full_addr = format!("{}/p2p/{}", network1_addr, network1_peer_id);
    
    network2.connect(&network1_full_addr.parse().unwrap()).await.unwrap();
    
    // Wait for connection to establish
    sleep(Duration::from_millis(200)).await;
    
    // Setup message reception tracking
    let received_large_message = Arc::new(Mutex::new(false));
    let received_large_message_clone = received_large_message.clone();
    
    // Create a message handler for network1
    let handler = Arc::new(DefaultMessageHandler::new(
        1,
        "LargeMessageHandler".to_string(),
        move |message, _| {
            if let NetworkMessage::TransactionAnnouncement(tx) = message {
                if tx.transaction_id == "large_tx" && tx.data_hash.len() > 10000 {
                    let mut received = received_large_message_clone.blocking_lock();
                    *received = true;
                }
            }
            Ok(())
        }
    ));
    
    // Register the handler
    network1.register_message_handler("ledger.transaction", handler).await.unwrap();
    
    // Create a large message (approximately 20KB)
    let large_data_hash = "0".repeat(20000);
    let large_tx = TransactionAnnouncement {
        transaction_id: "large_tx".to_string(),
        transaction_type: "large_test".to_string(),
        timestamp: 1234567890,
        sender: "large_node".to_string(),
        data_hash: large_data_hash,
    };
    
    // Broadcast the large message
    network2.broadcast(NetworkMessage::TransactionAnnouncement(large_tx)).await.unwrap();
    
    // Wait for message propagation
    sleep(Duration::from_secs(2)).await;
    
    // Check that the large message was received
    let received = *received_large_message.lock().await;
    assert!(received, "Large message was not received");
    
    // Stop networks
    network1.stop().await.unwrap();
    network2.stop().await.unwrap();
}

#[tokio::test]
async fn test_network_reconnection() {
    init_logging();
    
    // Create two test networks
    let network1 = create_test_network(9201, "reconnect1", false, vec![]).await;
    let network2 = create_test_network(9202, "reconnect2", false, vec![]).await;
    
    // Start the networks
    network1.start().await.unwrap();
    network2.start().await.unwrap();
    
    // Wait for networks to initialize
    sleep(Duration::from_millis(100)).await;
    
    // Connect network2 to network1
    let network1_peer_id = network1.local_peer_id();
    let network1_addr = network1.listen_addresses().await.unwrap()[0].clone();
    let network1_full_addr = format!("{}/p2p/{}", network1_addr, network1_peer_id);
    
    network2.connect(&network1_full_addr.parse().unwrap()).await.unwrap();
    
    // Wait for connection to establish
    sleep(Duration::from_millis(200)).await;
    
    // Verify the connection
    let peers1 = network1.connected_peers().await.unwrap();
    let peers2 = network2.connected_peers().await.unwrap();
    
    assert_eq!(peers1.len(), 1, "Network 1 should have 1 connected peer");
    assert_eq!(peers2.len(), 1, "Network 2 should have 1 connected peer");
    
    // Disconnect and reconnect
    network2.disconnect(&network1_peer_id).await.unwrap();
    
    // Wait for disconnection
    sleep(Duration::from_millis(200)).await;
    
    // Verify disconnection
    let peers1 = network1.connected_peers().await.unwrap();
    let peers2 = network2.connected_peers().await.unwrap();
    
    assert_eq!(peers1.len(), 0, "Network 1 should have 0 connected peers after disconnect");
    assert_eq!(peers2.len(), 0, "Network 2 should have 0 connected peers after disconnect");
    
    // Reconnect
    network2.connect(&network1_full_addr.parse().unwrap()).await.unwrap();
    
    // Wait for reconnection
    sleep(Duration::from_millis(200)).await;
    
    // Verify reconnection
    let peers1 = network1.connected_peers().await.unwrap();
    let peers2 = network2.connected_peers().await.unwrap();
    
    assert_eq!(peers1.len(), 1, "Network 1 should have 1 connected peer after reconnect");
    assert_eq!(peers2.len(), 1, "Network 2 should have 1 connected peer after reconnect");
    
    // Stop networks
    network1.stop().await.unwrap();
    network2.stop().await.unwrap();
} 