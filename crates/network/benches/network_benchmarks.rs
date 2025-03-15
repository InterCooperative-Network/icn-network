use std::sync::Arc;
use std::time::{Duration, Instant};

use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId, BatchSize};
use futures::future::join_all;
use tokio::runtime::Runtime;
use tokio::sync::Mutex;

use icn_core::storage::mock_storage::MockStorage;
use icn_network::{
    P2pNetwork, P2pConfig, MessageProcessor, NetworkMessage,
    TransactionAnnouncement, DefaultMessageHandler, PeerInfo,
    NetworkResult,
};
use libp2p::Multiaddr;

/// Create a test network for benchmarking
async fn setup_test_network(port: u16) -> Arc<P2pNetwork> {
    let storage = Arc::new(MockStorage::new());
    
    let mut config = P2pConfig::default();
    config.listen_addresses = vec![format!("/ip4/127.0.0.1/tcp/{}", port).parse().unwrap()];
    config.enable_mdns = false; // Disable mDNS for benchmarks
    
    let network = P2pNetwork::new(storage, config).await.unwrap();
    Arc::new(network)
}

/// Connect two networks together
async fn connect_networks(network1: &Arc<P2pNetwork>, network2: &Arc<P2pNetwork>) -> NetworkResult<()> {
    // Start both networks
    network1.start().await?;
    network2.start().await?;
    
    // Wait for the addresses to be available
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Get node 1's address
    let node1_peer_id = network1.local_peer_id();
    let node1_listen_addr = network1.listen_addresses().await?[0].clone();
    
    // Create a multiaddr for node 1 that includes the peer ID
    let node1_addr = format!("{}/p2p/{}", node1_listen_addr, node1_peer_id)
        .parse::<Multiaddr>()
        .unwrap();
    
    // Connect node 2 to node 1
    network2.connect(&node1_addr).await?;
    
    Ok(())
}

/// Benchmark message broadcasting
fn bench_broadcast(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("network_broadcast");
    
    // Benchmark different message sizes
    for size in [1, 10, 100, 1000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter_batched(
                || {
                    // Setup code that is not measured
                    rt.block_on(async {
                        // Create two networks
                        let network1 = setup_test_network(10001).await;
                        let network2 = setup_test_network(10002).await;
                        
                        // Connect the networks
                        connect_networks(&network1, &network2).await.unwrap();
                        
                        // Create a message handler for network 1
                        let received_message = Arc::new(Mutex::new(false));
                        let received_message_clone = received_message.clone();
                        
                        let handler = Arc::new(DefaultMessageHandler::new(
                            1,
                            "BenchHandler".to_string(),
                            move |message, _| {
                                if let NetworkMessage::TransactionAnnouncement(_) = message {
                                    let mut received = received_message_clone.blocking_lock();
                                    *received = true;
                                }
                                
                                Ok(())
                            }
                        ));
                        
                        // Register the handler
                        network1.register_message_handler("ledger.transaction", handler).await.unwrap();
                        
                        // Generate data for the message (simulate different sizes)
                        let data_hash = "0".repeat(size);
                        
                        (network1, network2, received_message, data_hash)
                    })
                },
                |(network1, network2, received_message, data_hash)| {
                    // The actual code being measured
                    rt.block_on(async {
                        // Reset the received flag
                        let mut received = received_message.lock().await;
                        *received = false;
                        drop(received);
                        
                        // Create the message
                        let tx_announce = TransactionAnnouncement {
                            transaction_id: "bench_tx".to_string(),
                            transaction_type: "transfer".to_string(),
                            timestamp: 12345,
                            sender: "bench_sender".to_string(),
                            data_hash,
                        };
                        
                        let message = NetworkMessage::TransactionAnnouncement(tx_announce);
                        
                        // Send the message
                        let start = Instant::now();
                        network2.broadcast(message).await.unwrap();
                        
                        // Wait for the message to be received
                        let mut received = false;
                        for _ in 0..100 {
                            tokio::time::sleep(Duration::from_millis(1)).await;
                            
                            if *received_message.lock().await {
                                received = true;
                                break;
                            }
                        }
                        
                        assert!(received, "Message was not received during benchmark");
                        
                        // Measure the time it took to receive the message
                        start.elapsed()
                    })
                },
                BatchSize::SmallInput,
            );
        });
    }
    
    group.finish();
}

/// Benchmark connecting to multiple peers
fn bench_connect_peers(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("network_connect");
    
    // Benchmark connecting to different numbers of peers
    for &num_peers in &[1, 5, 10] {
        group.bench_with_input(BenchmarkId::from_parameter(num_peers), &num_peers, |b, &num_peers| {
            b.iter_batched(
                || {
                    // Setup code that is not measured
                    rt.block_on(async {
                        // Create the hub network
                        let hub_network = setup_test_network(11000).await;
                        hub_network.start().await.unwrap();
                        
                        // Wait for the addresses to be available
                        tokio::time::sleep(Duration::from_millis(100)).await;
                        
                        // Get hub's address
                        let hub_peer_id = hub_network.local_peer_id();
                        let hub_listen_addr = hub_network.listen_addresses().await.unwrap()[0].clone();
                        
                        // Create a multiaddr for hub that includes the peer ID
                        let hub_addr = format!("{}/p2p/{}", hub_listen_addr, hub_peer_id)
                            .parse::<Multiaddr>()
                            .unwrap();
                        
                        // Create the satellite networks
                        let mut satellite_networks = Vec::with_capacity(num_peers as usize);
                        for i in 0..num_peers {
                            let network = setup_test_network(11001 + i).await;
                            network.start().await.unwrap();
                            satellite_networks.push(network);
                        }
                        
                        (hub_network, satellite_networks, hub_addr)
                    })
                },
                |(hub_network, satellite_networks, hub_addr)| {
                    // The actual code being measured
                    rt.block_on(async {
                        let start = Instant::now();
                        
                        // Connect all satellites to the hub
                        let connects = satellite_networks.iter().map(|network| {
                            network.connect(&hub_addr)
                        });
                        
                        // Wait for all connections to complete
                        let results = join_all(connects).await;
                        
                        // Verify all connections succeeded
                        for result in results {
                            assert!(result.is_ok(), "Connection failed");
                        }
                        
                        // Measure the time it took to connect all peers
                        start.elapsed()
                    })
                },
                BatchSize::SmallInput,
            );
        });
    }
    
    group.finish();
}

/// Benchmark message throughput
fn bench_message_throughput(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("network_throughput");
    
    // Benchmark different numbers of messages
    for &num_messages in &[10, 100, 1000] {
        group.bench_with_input(BenchmarkId::from_parameter(num_messages), &num_messages, |b, &num_messages| {
            b.iter_batched(
                || {
                    // Setup code that is not measured
                    rt.block_on(async {
                        // Create two networks
                        let network1 = setup_test_network(12001).await;
                        let network2 = setup_test_network(12002).await;
                        
                        // Connect the networks
                        connect_networks(&network1, &network2).await.unwrap();
                        
                        // Create a counter for received messages
                        let received_count = Arc::new(Mutex::new(0));
                        let received_count_clone = received_count.clone();
                        
                        let handler = Arc::new(DefaultMessageHandler::new(
                            1,
                            "ThroughputHandler".to_string(),
                            move |message, _| {
                                if let NetworkMessage::TransactionAnnouncement(_) = message {
                                    let mut count = received_count_clone.blocking_lock();
                                    *count += 1;
                                }
                                
                                Ok(())
                            }
                        ));
                        
                        // Register the handler
                        network1.register_message_handler("ledger.transaction", handler).await.unwrap();
                        
                        // Generate messages
                        let mut messages = Vec::with_capacity(num_messages as usize);
                        for i in 0..num_messages {
                            let tx_announce = TransactionAnnouncement {
                                transaction_id: format!("throughput_tx_{}", i),
                                transaction_type: "transfer".to_string(),
                                timestamp: 12345,
                                sender: "throughput_sender".to_string(),
                                data_hash: "throughput_hash".to_string(),
                            };
                            
                            messages.push(NetworkMessage::TransactionAnnouncement(tx_announce));
                        }
                        
                        (network1, network2, received_count, messages)
                    })
                },
                |(network1, network2, received_count, messages)| {
                    // The actual code being measured
                    rt.block_on(async {
                        // Reset the counter
                        let mut count = received_count.lock().await;
                        *count = 0;
                        drop(count);
                        
                        let start = Instant::now();
                        
                        // Send all messages in rapid succession
                        for message in messages {
                            network2.broadcast(message).await.unwrap();
                        }
                        
                        // Wait until all messages are received or timeout
                        let timeout = Duration::from_secs(10);
                        let start_wait = Instant::now();
                        
                        loop {
                            let count = *received_count.lock().await;
                            if count >= num_messages as usize {
                                break;
                            }
                            
                            if start_wait.elapsed() > timeout {
                                panic!("Timeout waiting for messages. Received {} of {}", count, num_messages);
                            }
                            
                            tokio::time::sleep(Duration::from_millis(10)).await;
                        }
                        
                        // Measure the time it took to send and receive all messages
                        start.elapsed()
                    })
                },
                BatchSize::SmallInput,
            );
        });
    }
    
    group.finish();
}

criterion_group!(
    benches,
    bench_broadcast,
    bench_connect_peers,
    bench_message_throughput
);
criterion_main!(benches); 