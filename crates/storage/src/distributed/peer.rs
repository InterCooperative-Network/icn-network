use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Information about a storage peer in the network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoragePeer {
    /// Unique identifier for the peer
    pub node_id: String,
    /// Network address of the peer
    pub address: String,
    /// Federation ID the peer belongs to
    pub federation_id: String,
    /// Total storage capacity in bytes
    pub storage_capacity: u64,
    /// Currently available space in bytes
    pub available_space: u64,
    /// Average latency to this peer in milliseconds
    pub latency_ms: u32,
    /// Uptime percentage (0-100)
    pub uptime_percentage: f32,
    /// Additional metadata tags
    pub tags: HashMap<String, String>,
}

impl StoragePeer {
    /// Create a new storage peer
    pub fn new(
        node_id: String,
        address: String,
        federation_id: String,
        storage_capacity: u64,
        available_space: u64,
    ) -> Self {
        Self {
            node_id,
            address,
            federation_id,
            storage_capacity,
            available_space,
            latency_ms: 0,
            uptime_percentage: 100.0,
            tags: HashMap::new(),
        }
    }
    
    /// Calculate a score for this peer based on various metrics
    pub fn calculate_score(&self, preferred_federation_id: Option<&str>) -> f32 {
        let mut score = 0.0;
        
        // Score based on available space (0-40 points)
        let space_ratio = self.available_space as f32 / self.storage_capacity as f32;
        score += space_ratio * 40.0;
        
        // Score based on latency (0-30 points)
        // Lower latency is better, max score at 0ms, min score at 1000ms
        let latency_score = ((1000.0 - self.latency_ms as f32) / 1000.0).max(0.0) * 30.0;
        score += latency_score;
        
        // Score based on uptime (0-20 points)
        score += self.uptime_percentage * 0.2;
        
        // Federation preference bonus (10 points)
        if let Some(preferred_id) = preferred_federation_id {
            if self.federation_id == preferred_id {
                score += 10.0;
            }
        }
        
        score
    }
    
    /// Update peer metrics
    pub fn update_metrics(&mut self, latency_ms: u32, uptime_percentage: f32) {
        self.latency_ms = latency_ms;
        self.uptime_percentage = uptime_percentage;
    }
    
    /// Update available space
    pub fn update_available_space(&mut self, available_space: u64) {
        self.available_space = available_space;
    }
    
    /// Add or update a tag
    pub fn set_tag(&mut self, key: String, value: String) {
        self.tags.insert(key, value);
    }
    
    /// Remove a tag
    pub fn remove_tag(&mut self, key: &str) -> Option<String> {
        self.tags.remove(key)
    }
    
    /// Check if peer has sufficient space
    pub fn has_sufficient_space(&self, required_space: u64) -> bool {
        self.available_space >= required_space
    }
    
    /// Get the federation ID
    pub fn federation_id(&self) -> &str {
        &self.federation_id
    }
    
    /// Get the node ID
    pub fn node_id(&self) -> &str {
        &self.node_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_peer_score_calculation() {
        let mut peer = StoragePeer::new(
            "node1".to_string(),
            "127.0.0.1:8000".to_string(),
            "fed1".to_string(),
            1000,
            800,
        );
        
        // Test with good metrics
        peer.update_metrics(50, 99.9);
        let score = peer.calculate_score(Some("fed1"));
        assert!(score > 80.0); // High score for good metrics
        
        // Test with poor metrics
        peer.update_metrics(500, 50.0);
        let score = peer.calculate_score(Some("fed1"));
        assert!(score < 60.0); // Lower score for poor metrics
        
        // Test federation preference
        let score_preferred = peer.calculate_score(Some("fed1"));
        let score_other = peer.calculate_score(Some("fed2"));
        assert!(score_preferred > score_other); // Preferred federation gets higher score
    }
    
    #[test]
    fn test_peer_space_management() {
        let mut peer = StoragePeer::new(
            "node1".to_string(),
            "127.0.0.1:8000".to_string(),
            "fed1".to_string(),
            1000,
            800,
        );
        
        assert!(peer.has_sufficient_space(500));
        assert!(!peer.has_sufficient_space(900));
        
        peer.update_available_space(1000);
        assert!(peer.has_sufficient_space(900));
    }
    
    #[test]
    fn test_peer_tags() {
        let mut peer = StoragePeer::new(
            "node1".to_string(),
            "127.0.0.1:8000".to_string(),
            "fed1".to_string(),
            1000,
            800,
        );
        
        peer.set_tag("region".to_string(), "us-west".to_string());
        assert_eq!(peer.tags.get("region").unwrap(), "us-west");
        
        peer.remove_tag("region");
        assert!(peer.tags.get("region").is_none());
    }
} 