use crate::reputation::{ReputationManager, ReputationConfig, ReputationChange};
use libp2p::PeerId;

#[tokio::test]
async fn test_reputation_basic() {
    let config = ReputationConfig::default();
    let manager = ReputationManager::new(config, None, None).await.unwrap();
    let peer_id = PeerId::random();
    
    // Record a change
    let score = manager.record_change(peer_id, ReputationChange::ConnectionEstablished).await.unwrap();
    println!("Score after connection established: {}", score);
    
    // Get the reputation
    let rep = manager.get_reputation(peer_id).await.unwrap();
    println!("Current reputation: {}", rep);
    
    // Check if banned
    let banned = manager.is_banned(peer_id).await;
    println!("Is banned: {}", banned);
} 