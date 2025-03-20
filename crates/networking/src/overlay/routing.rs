use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use serde::{Serialize, Deserialize};
use crate::error::{Result, NetworkError};
use crate::overlay::address::OverlayAddress;

/// Route information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteInfo {
    /// Destination address
    pub destination: OverlayAddress,
    /// Next hop address
    pub next_hop: OverlayAddress,
    /// Number of hops to destination
    pub hop_count: u8,
    /// Route metric (lower is better)
    pub metric: u32,
    /// Route is valid
    pub valid: bool,
    /// Route expiration timestamp
    pub expires_at: u64,
}

/// Routing table for the overlay network
#[derive(Debug)]
pub struct RoutingTable {
    /// Routes indexed by destination address
    routes: HashMap<OverlayAddress, RouteInfo>,
    /// Default route
    default_route: Option<RouteInfo>,
}

impl RoutingTable {
    /// Create a new routing table
    pub fn new() -> Self {
        Self {
            routes: HashMap::new(),
            default_route: None,
        }
    }
    
    /// Add a route to the routing table
    pub fn add_route(&mut self, route: RouteInfo) {
        self.routes.insert(route.destination.clone(), route);
    }
    
    /// Remove a route from the routing table
    pub fn remove_route(&mut self, destination: &OverlayAddress) -> Option<RouteInfo> {
        self.routes.remove(destination)
    }
    
    /// Get a route for the given destination
    pub fn get_route(&self, destination: &OverlayAddress) -> Option<RouteInfo> {
        self.routes.get(destination).cloned()
    }
    
    /// Set the default route
    pub fn set_default_route(&mut self, route: RouteInfo) {
        self.default_route = Some(route);
    }
    
    /// Get the default route
    pub fn get_default_route(&self) -> Option<RouteInfo> {
        self.default_route.clone()
    }
    
    /// Get all routes
    pub fn get_all_routes(&self) -> Vec<RouteInfo> {
        self.routes.values().cloned().collect()
    }
}

/// Route manager for finding and managing routes
pub struct RouteManager {
    /// Local routing table
    routing_table: Arc<RwLock<RoutingTable>>,
    /// Local address
    local_address: Option<OverlayAddress>,
}

impl RouteManager {
    /// Create a new route manager
    pub fn new() -> Self {
        Self {
            routing_table: Arc::new(RwLock::new(RoutingTable::new())),
            local_address: None,
        }
    }
    
    /// Initialize with local address
    pub fn initialize(&mut self, local_address: OverlayAddress) {
        self.local_address = Some(local_address);
    }
    
    /// Add a route
    pub fn add_route(&self, route: RouteInfo) -> Result<()> {
        let mut table = self.routing_table.write().unwrap();
        table.add_route(route);
        Ok(())
    }
    
    /// Find a route to the destination
    pub async fn find_route(&self, source: &OverlayAddress, destination: &OverlayAddress) -> Result<RouteInfo> {
        // Check if destination is self
        if let Some(local) = &self.local_address {
            if destination == local {
                return Ok(RouteInfo {
                    destination: destination.clone(),
                    next_hop: destination.clone(),
                    hop_count: 0,
                    metric: 0,
                    valid: true,
                    expires_at: u64::MAX, // Never expires
                });
            }
        }
        
        // Look up in routing table
        let table = self.routing_table.read().unwrap();
        if let Some(route) = table.get_route(destination) {
            return Ok(route);
        }
        
        // Use default route if available
        if let Some(default_route) = table.get_default_route() {
            return Ok(RouteInfo {
                destination: destination.clone(),
                next_hop: default_route.next_hop,
                hop_count: default_route.hop_count + 1,
                metric: default_route.metric + 10, // Higher metric for default route
                valid: true,
                expires_at: 0, // Expires immediately (will need to be refreshed)
            });
        }
        
        // No route found
        Err(NetworkError::RoutingError(format!(
            "No route to destination: {}",
            destination
        )))
    }
    
    /// Get all routes
    pub fn get_all_routes(&self) -> Result<Vec<RouteInfo>> {
        let table = self.routing_table.read().unwrap();
        Ok(table.get_all_routes())
    }
    
    /// Remove expired routes
    pub fn cleanup_expired_routes(&self, current_time: u64) -> Result<usize> {
        let mut table = self.routing_table.write().unwrap();
        let mut count = 0;
        
        let expired_destinations: Vec<OverlayAddress> = table
            .routes
            .iter()
            .filter(|(_, route)| route.expires_at < current_time && route.expires_at > 0)
            .map(|(addr, _)| addr.clone())
            .collect();
            
        for addr in expired_destinations {
            table.remove_route(&addr);
            count += 1;
        }
        
        Ok(count)
    }
}