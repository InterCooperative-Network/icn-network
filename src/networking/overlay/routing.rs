//! Overlay network routing
//! 
//! This module provides routing functionality for the overlay network.

use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use tracing::{debug, info};

use crate::error::{Result, NetworkError};
use super::address::OverlayAddress;

/// Information about a route in the overlay network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteInfo {
    /// Destination address
    pub destination: OverlayAddress,
    /// Next hop to reach the destination (None for direct delivery)
    pub next_hop: Option<OverlayAddress>,
    /// Complete path to the destination
    pub path: Vec<OverlayAddress>,
    /// Cost metric for the route
    pub cost: u32,
    /// Timestamp when the route was last updated
    pub last_updated: i64,
}

/// Routing table for the overlay network
pub struct RoutingTable {
    /// Routes to specific destinations
    pub routes: HashMap<OverlayAddress, RouteInfo>,
    /// Routes to federations
    pub federation_routes: HashMap<String, Vec<RouteInfo>>,
    /// Local address
    pub local_address: Option<OverlayAddress>,
}

/// Manages routing in the overlay network
pub struct RouteManager {
    /// Routing table
    routing_table: RoutingTable,
}

impl RouteManager {
    /// Create a new route manager
    pub fn new() -> Self {
        Self {
            routing_table: RoutingTable {
                routes: HashMap::new(),
                federation_routes: HashMap::new(),
                local_address: None,
            },
        }
    }
    
    /// Initialize the route manager
    pub fn initialize(&mut self, local_address: &OverlayAddress) -> Result<()> {
        // Add a route to self
        let self_route = RouteInfo {
            destination: local_address.clone(),
            next_hop: None, // Direct
            path: vec![local_address.clone()],
            cost: 0,
            last_updated: chrono::Utc::now().timestamp(),
        };
        
        self.routing_table.routes.insert(local_address.clone(), self_route);
        self.routing_table.local_address = Some(local_address.clone());
        
        // If part of a federation, add to federation routes
        if let Some(federation_id) = &local_address.federation {
            let routes = self.routing_table.federation_routes
                .entry(federation_id.clone())
                .or_insert_with(Vec::new);
            
            routes.push(RouteInfo {
                destination: local_address.clone(),
                next_hop: None,
                path: vec![local_address.clone()],
                cost: 0,
                last_updated: chrono::Utc::now().timestamp(),
            });
        }
        
        Ok(())
    }
    
    /// Find a route to a destination
    pub fn find_route(&self, destination: &OverlayAddress) -> Result<RouteInfo> {
        // Check if we have a direct route
        if let Some(route) = self.routing_table.routes.get(destination) {
            debug!("Found direct route to {:?}", destination);
            return Ok(route.clone());
        }
        
        // If destination is in a federation, check federation routes
        if let Some(federation_id) = &destination.federation {
            if let Some(routes) = self.routing_table.federation_routes.get(federation_id) {
                if !routes.is_empty() {
                    debug!("Found federation route to {:?} via federation {}", destination, federation_id);
                    // Use the first federation route as gateway
                    let gateway_route = &routes[0];
                    
                    return Ok(RouteInfo {
                        destination: destination.clone(),
                        next_hop: gateway_route.next_hop.clone(),
                        path: vec![gateway_route.destination.clone(), destination.clone()],
                        cost: gateway_route.cost + 1,
                        last_updated: chrono::Utc::now().timestamp(),
                    });
                }
            }
        }
        
        // No route found
        Err(NetworkError::Other(format!("No route to {:?}", destination)))
    }
    
    /// Get routes to a federation
    pub fn get_federation_routes(&self, federation_id: &str) -> Result<Vec<RouteInfo>> {
        if let Some(routes) = self.routing_table.federation_routes.get(federation_id) {
            return Ok(routes.clone());
        }
        
        Err(NetworkError::Other(format!("No routes to federation {}", federation_id)))
    }
    
    /// Add a route
    pub fn add_route(&mut self, route: RouteInfo) -> Result<()> {
        // Check if route already exists
        if let Some(existing_route) = self.routing_table.routes.get(&route.destination) {
            // Only update if new route is better
            if route.cost < existing_route.cost {
                self.routing_table.routes.insert(route.destination.clone(), route.clone());
                debug!("Updated route to {:?} with cost {}", route.destination, route.cost);
            }
        } else {
            // Add new route
            debug!("Added new route to {:?} with cost {}", route.destination, route.cost);
            self.routing_table.routes.insert(route.destination.clone(), route.clone());
        }
        
        // If destination is in a federation, update federation routes
        if let Some(federation_id) = &route.destination.federation {
            let routes = self.routing_table.federation_routes
                .entry(federation_id.clone())
                .or_insert_with(Vec::new);
            
            // Check if federation route already exists
            let existing_index = routes.iter()
                .position(|r| r.destination == route.destination);
            
            if let Some(index) = existing_index {
                // Only update if new route is better
                if route.cost < routes[index].cost {
                    routes[index] = route;
                }
            } else {
                // Add new federation route
                routes.push(route);
            }
        }
        
        Ok(())
    }
    
    /// Remove a route
    pub fn remove_route(&mut self, destination: &OverlayAddress) -> Result<()> {
        if self.routing_table.routes.remove(destination).is_some() {
            debug!("Removed route to {:?}", destination);
        }
        
        // Also remove from federation routes if applicable
        if let Some(federation_id) = &destination.federation {
            if let Some(routes) = self.routing_table.federation_routes.get_mut(federation_id) {
                routes.retain(|r| r.destination != *destination);
            }
        }
        
        Ok(())
    }
    
    /// Update route costs based on network conditions
    pub fn update_route_costs(&mut self) -> Result<()> {
        // In a real implementation, this would update costs based on latency, bandwidth, etc.
        // For now, just update timestamps
        let now = chrono::Utc::now().timestamp();
        
        for route in self.routing_table.routes.values_mut() {
            route.last_updated = now;
        }
        
        for routes in self.routing_table.federation_routes.values_mut() {
            for route in routes {
                route.last_updated = now;
            }
        }
        
        Ok(())
    }
}
