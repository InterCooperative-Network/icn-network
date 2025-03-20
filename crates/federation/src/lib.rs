pub mod api;
pub mod models;
pub mod services;
pub mod utils;

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use std::sync::Arc;

/// Federation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationConfig {
    pub name: String,
    pub description: String,
    pub federation_id: String,
    pub peers: Vec<String>,
    pub storage_path: String,
}

/// Federation state
#[derive(Debug)]
pub struct Federation {
    config: FederationConfig,
    state: Arc<RwLock<FederationState>>,
}

#[derive(Debug, Default)]
struct FederationState {
    is_active: bool,
    connected_peers: Vec<String>,
    resource_usage: std::collections::HashMap<String, f64>,
}

impl Federation {
    pub async fn new(config: FederationConfig) -> Result<Self> {
        Ok(Self {
            config,
            state: Arc::new(RwLock::new(FederationState::default())),
        })
    }

    pub async fn start(&self) -> Result<()> {
        let mut state = self.state.write().await;
        state.is_active = true;
        Ok(())
    }

    pub async fn stop(&self) -> Result<()> {
        let mut state = self.state.write().await;
        state.is_active = false;
        Ok(())
    }

    pub async fn is_active(&self) -> bool {
        self.state.read().await.is_active
    }

    pub async fn connect_peer(&self, peer_id: String) -> Result<()> {
        let mut state = self.state.write().await;
        if !state.connected_peers.contains(&peer_id) {
            state.connected_peers.push(peer_id);
        }
        Ok(())
    }

    pub async fn disconnect_peer(&self, peer_id: &str) -> Result<()> {
        let mut state = self.state.write().await;
        state.connected_peers.retain(|p| p != peer_id);
        Ok(())
    }

    pub async fn get_connected_peers(&self) -> Vec<String> {
        self.state.read().await.connected_peers.clone()
    }
}

#[async_trait]
pub trait FederationService {
    async fn join_federation(&self, federation_id: String) -> Result<()>;
    async fn leave_federation(&self, federation_id: String) -> Result<()>;
    async fn get_federation_info(&self, federation_id: String) -> Result<FederationConfig>;
    async fn list_federations(&self) -> Result<Vec<String>>;
}

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}

