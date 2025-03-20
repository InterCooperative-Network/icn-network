//! Federation storage router for ICN
//!
//! This module provides storage routing functionality for federated storage systems:
//! - Federation-specific storage routing
//! - Storage route management
//! - Routing strategies
//! - Federation storage access control

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};
use thiserror::Error;

/// Federation storage router
pub struct FederationStorageRouter {
    /// Router configuration
    config: RouterConfig,
    /// Storage routes
    routes: Arc<RwLock<HashMap<String, StorageRoute>>>,
}

/// Router configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RouterConfig {
    /// Default routing strategy
    pub default_strategy: RoutingStrategy,
    /// Federation ID
    pub federation_id: String,
    /// Maximum number of routes
    pub max_routes: usize,
}

/// Storage route
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StorageRoute {
    /// Route ID
    pub id: String,
    /// Federation ID
    pub federation_id: String,
    /// Source location
    pub source: String,
    /// Destination location
    pub destination: String,
    /// Routing strategy
    pub strategy: RoutingStrategy,
}

/// Routing strategy
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum RoutingStrategy {
    /// Round robin routing
    RoundRobin,
    /// Random routing
    Random,
    /// Weighted routing
    Weighted(HashMap<String, u32>),
    /// Proximity-based routing
    Proximity,
    /// Capacity-based routing
    Capacity,
    /// Custom routing
    Custom(String),
}

/// Router errors
#[derive(Debug, Error)]
pub enum RouterError {
    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigError(String),
    /// Routing error
    #[error("Routing error: {0}")]
    RoutingError(String),
    /// Federation error
    #[error("Federation error: {0}")]
    FederationError(String),
    /// Storage error
    #[error("Storage error: {0}")]
    StorageError(String),
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self {
            default_strategy: RoutingStrategy::RoundRobin,
            federation_id: "default".to_string(),
            max_routes: 1000,
        }
    }
}

impl FederationStorageRouter {
    /// Create a new router with the given configuration
    pub fn new(config: RouterConfig) -> Self {
        Self {
            config,
            routes: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Add a new route
    pub async fn add_route(&self, route: StorageRoute) -> Result<(), RouterError> {
        let mut routes = self.routes.write().await;
        if routes.len() >= self.config.max_routes {
            return Err(RouterError::ConfigError("Maximum number of routes reached".to_string()));
        }
        routes.insert(route.id.clone(), route);
        Ok(())
    }
    
    /// Get a route by ID
    pub async fn get_route(&self, id: &str) -> Option<StorageRoute> {
        let routes = self.routes.read().await;
        routes.get(id).cloned()
    }
    
    /// Remove a route
    pub async fn remove_route(&self, id: &str) -> Result<(), RouterError> {
        let mut routes = self.routes.write().await;
        if routes.remove(id).is_none() {
            return Err(RouterError::RoutingError(format!("Route not found: {}", id)));
        }
        Ok(())
    }
    
    /// Get all routes
    pub async fn get_all_routes(&self) -> Vec<StorageRoute> {
        let routes = self.routes.read().await;
        routes.values().cloned().collect()
    }
} 