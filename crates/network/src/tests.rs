//! Tests for the network crate
//!
//! This module contains integration tests for the network components,
//! testing P2P communication, message handling, and peer discovery.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Mutex;
use tokio::time::sleep;
use libp2p::{Multiaddr, PeerId};

use icn_core::storage::mock_storage::MockStorage;

use crate::{
    P2pNetwork, P2pConfig, 
    MessageProcessor, NetworkMessage, DefaultMessageHandler,
    TransactionAnnouncement, PeerInfo, NetworkResult, 
    DiscoveryManager, DiscoveryConfig, 
    Synchronizer, SyncConfig
};

/// Test configuration for a network node
async fn setup_test_network(port: u16) -> Arc<P2pNetwork> {
    let storage = Arc::new(MockStorage::new());
    
    let mut config = P2pConfig::default();
    config.listen_addresses = vec![format!("/ip4/127.0.0.1/tcp/{}", port).parse().unwrap()];
    config.enable_mdns = false; // Disable mDNS for tests
    
    let network = P2pNetwork::new(storage, config).await.unwrap();
    Arc::new(network)
}

#[tokio::test]
async fn test_network_start_stop() {
    let network = setup_test_network(9010).await;
    
    // Start the network
    network.start().await.unwrap();
    
    // Check that we can get the local peer ID
    let peer_id = network.local_peer_id();
    assert!(!peer_id.to_string().is_empty());
    
    // Check that we are listening
    let addrs = network.listen_addresses().await.unwrap();
    assert!(!addrs.is_empty());
    
    // Stop the network
    network.stop().await.unwrap();
}

#[tokio::test]
async fn test_network_connect_disconnect() {
    // Create two networks
    let network1 = setup_test_network(9011).await;
    let network2 = setup_test_network(9012).await;
    
    // Start both networks
    network1.start().await.unwrap();
    network2.start().await.unwrap();
    
    // Wait for the addresses to be available
    sleep(Duration::from_millis(100)).await;
    
    // Get node 1's address
    let node1_peer_id = network1.local_peer_id();
    let node1_listen_addr = network1.listen_addresses().await.unwrap()[0].clone();
    
    // Create a multiaddr for node 1 that includes the peer ID
    let node1_addr = format!("{}/p2p/{}", node1_listen_addr, node1_peer_id)
        .parse::<Multiaddr>()
        .unwrap();
    
    // Connect node 2 to node 1
    let peer_id = network2.connect(&node1_addr).await.unwrap();
    assert_eq!(peer_id, node1_peer_id);
    
    // Check that node 2 sees node 1 as connected
    let peers = network2.get_connected_peers().await.unwrap();
    assert!(!peers.is_empty());
    
    // Disconnect from node 1
    network2.disconnect(&node1_peer_id).await.unwrap();
    
    // Give some time for the disconnect to propagate
    sleep(Duration::from_millis(100)).await;
    
    // Check that node 2 no longer sees node 1 as connected
    let peers = network2.get_connected_peers().await.unwrap();
    assert!(peers.iter().find(|p| p.peer_id == node1_peer_id).is_none());
    
    // Clean up
    network1.stop().await.unwrap();
    network2.stop().await.unwrap();
}

#[tokio::test]
async fn test_message_handlers() {
    // Create two networks
    let network1 = setup_test_network(9021).await;
    let network2 = setup_test_network(9022).await;
    
    // Start both networks
    network1.start().await.unwrap();
    network2.start().await.unwrap();
    
    // Wait for the addresses to be available
    sleep(Duration::from_millis(100)).await;
    
    // Get node 1's address
    let node1_peer_id = network1.local_peer_id();
    let node1_listen_addr = network1.listen_addresses().await.unwrap()[0].clone();
    
    // Create a multiaddr for node 1 that includes the peer ID
    let node1_addr = format!("{}/p2p/{}", node1_listen_addr, node1_peer_id)
        .parse::<Multiaddr>()
        .unwrap();
    
    // Connect node 2 to node 1
    network2.connect(&node1_addr).await.unwrap();
    
    // Create a flag to check when a message is received
    let received_message = Arc::new(Mutex::new(false));
    let received_message_clone = received_message.clone();
    
    // Create a message handler for node 1
    let handler = Arc::new(DefaultMessageHandler::new(
        1,
        "TestHandler".to_string(),
        move |message, peer| {
            if let NetworkMessage::TransactionAnnouncement(tx) = message {
                let mut received = received_message_clone.blocking_lock();
                *received = true;
            }
            
            Ok(())
        }
    ));
    
    // Register the handler with node 1
    network1.register_message_handler("ledger.transaction", handler).await.unwrap();
    
    // Create a test message
    let tx_announce = TransactionAnnouncement {
        transaction_id: "tx123".to_string(),
        transaction_type: "transfer".to_string(),
        timestamp: 12345,
        sender: "alice".to_string(),
        data_hash: "abcdef123456".to_string(),
    };
    
    let message = NetworkMessage::TransactionAnnouncement(tx_announce);
    
    // Send the message from node 2 to node 1
    network2.broadcast(message).await.unwrap();
    
    // Wait for the message to be received (with timeout)
    let mut message_received = false;
    for _ in 0..10 {
        sleep(Duration::from_millis(100)).await;
        
        let received = *received_message.lock().await;
        if received {
            message_received = true;
            break;
        }
    }
    
    // Check that the message was received
    assert!(message_received, "Message was not received");
    
    // Clean up
    network1.stop().await.unwrap();
    network2.stop().await.unwrap();
}

#[tokio::test]
async fn test_discovery_manager() {
    // Create two networks
    let network1 = setup_test_network(9031).await;
    let network2 = setup_test_network(9032).await;
    
    // Start both networks
    network1.start().await.unwrap();
    network2.start().await.unwrap();
    
    // Wait for the addresses to be available
    sleep(Duration::from_millis(100)).await;
    
    // Get node 1's address
    let node1_listen_addr = network1.listen_addresses().await.unwrap()[0].clone();
    
    // Create discovery manager for node 2 that uses node 1 as bootstrap
    let storage2 = Arc::new(MockStorage::new());
    let mut discovery_config = DiscoveryConfig::default();
    discovery_config.bootstrap_peers = vec![node1_listen_addr.clone()];
    discovery_config.use_mdns = false;
    discovery_config.use_kademlia = false;
    
    let discovery = DiscoveryManager::new(
        network2.clone(),
        storage2.clone(),
        discovery_config,
    );
    
    // Start discovery
    discovery.start().await.unwrap();
    
    // Wait for discovery to find peers
    sleep(Duration::from_millis(500)).await;
    
    // Check that node 2 has discovered node 1
    let discovered = discovery.get_discovered_peers().await.unwrap();
    assert!(!discovered.is_empty(), "No peers discovered");
    
    // Clean up
    discovery.stop().await.unwrap();
    network1.stop().await.unwrap();
    network2.stop().await.unwrap();
}

#[tokio::test]
async fn test_synchronizer() {
    // Create two networks
    let network1 = setup_test_network(9041).await;
    let network2 = setup_test_network(9042).await;
    
    // Start both networks
    network1.start().await.unwrap();
    network2.start().await.unwrap();
    
    // Wait for the addresses to be available
    sleep(Duration::from_millis(100)).await;
    
    // Get node 1's address
    let node1_peer_id = network1.local_peer_id();
    let node1_listen_addr = network1.listen_addresses().await.unwrap()[0].clone();
    
    // Create a multiaddr for node 1 that includes the peer ID
    let node1_addr = format!("{}/p2p/{}", node1_listen_addr, node1_peer_id)
        .parse::<Multiaddr>()
        .unwrap();
    
    // Connect node 2 to node 1
    network2.connect(&node1_addr).await.unwrap();
    
    // Create synchronizer for node 2
    let storage2 = Arc::new(MockStorage::new());
    let sync_config = SyncConfig::default();
    
    let synchronizer = Synchronizer::new(
        storage2.clone(),
        network2.clone(),
        sync_config,
    );
    
    // Start synchronizer
    synchronizer.start().await.unwrap();
    
    // Force sync now
    synchronizer.sync_now().await.unwrap();
    
    // Wait for sync to complete
    sleep(Duration::from_millis(500)).await;
    
    // Check the sync state
    let state = synchronizer.get_state().await;
    assert_eq!(state, crate::SyncState::Idle, "Sync did not complete");
    
    // Clean up
    synchronizer.stop().await.unwrap();
    network1.stop().await.unwrap();
    network2.stop().await.unwrap();
}

#[tokio::test]
async fn test_direct_messaging() {
    // Create two networks
    let network1 = setup_test_network(9051).await;
    let network2 = setup_test_network(9052).await;
    
    // Start both networks
    network1.start().await.unwrap();
    network2.start().await.unwrap();
    
    // Wait for the addresses to be available
    sleep(Duration::from_millis(100)).await;
    
    // Get node 1's address
    let node1_peer_id = network1.local_peer_id();
    let node1_listen_addr = network1.listen_addresses().await.unwrap()[0].clone();
    
    // Create a multiaddr for node 1 that includes the peer ID
    let node1_addr = format!("{}/p2p/{}", node1_listen_addr, node1_peer_id)
        .parse::<Multiaddr>()
        .unwrap();
    
    // Connect node 2 to node 1
    network2.connect(&node1_addr).await.unwrap();
    
    // Create a flag to check when a message is received
    let received_message = Arc::new(Mutex::new(false));
    let received_message_clone = received_message.clone();
    
    // Create a message handler for node 1
    let handler = Arc::new(DefaultMessageHandler::new(
        1,
        "TestHandler".to_string(),
        move |message, peer| {
            if let NetworkMessage::TransactionAnnouncement(tx) = message {
                let mut received = received_message_clone.blocking_lock();
                *received = true;
            }
            
            Ok(())
        }
    ));
    
    // Register the handler with node 1
    network1.register_message_handler("ledger.transaction", handler).await.unwrap();
    
    // Create a test message
    let tx_announce = TransactionAnnouncement {
        transaction_id: "tx456".to_string(),
        transaction_type: "transfer".to_string(),
        timestamp: 12345,
        sender: "bob".to_string(),
        data_hash: "fedcba654321".to_string(),
    };
    
    let message = NetworkMessage::TransactionAnnouncement(tx_announce);
    
    // Send the message directly from node 2 to node 1
    network2.send_to(&node1_peer_id, message).await.unwrap();
    
    // Wait for the message to be received (with timeout)
    let mut message_received = false;
    for _ in 0..10 {
        sleep(Duration::from_millis(100)).await;
        
        let received = *received_message.lock().await;
        if received {
            message_received = true;
            break;
        }
    }
    
    // Check that the message was received
    assert!(message_received, "Message was not received");
    
    // Clean up
    network1.stop().await.unwrap();
    network2.stop().await.unwrap();
} 