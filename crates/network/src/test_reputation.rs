use crate::reputation::{ReputationManager, ReputationConfig, ReputationChange, ReputationContext};
use libp2p::PeerId;

#[tokio::test]
async fn test_reputation_basic() {
    let config = ReputationConfig::default();
    let manager = ReputationManager::new(config);
    let peer_id = PeerId::random();
    
    // Record a change
    manager.record_change(peer_id, ReputationChange::Positive(10)).await.unwrap();
    
    // Get the reputation
    let rep = manager.get_reputation_async(&peer_id, &ReputationContext::Networking).await;
    println!("Current reputation: {}", rep);
    
    // Check if banned
    let banned = manager.is_banned(&peer_id);
    println!("Is banned: {}", banned);
} 