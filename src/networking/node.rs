// ...existing code...

use super::overlay::{
    OverlayNetworkManager, OverlayNetworkService, OverlayAddress, 
    OverlayOptions, MessagePriority
};

// ...existing code...

pub struct Node {
    // ...existing code...
    
    /// Overlay network manager
    overlay: OverlayNetworkManager,
    
    /// Overlay network address
    overlay_address: Option<OverlayAddress>,
    
    // ...existing code...
}

impl Node {
    pub fn new(id: NodeId, config: NodeConfig) -> Self {
        // ...existing code...
        
        Self {
            // ...existing code...
            overlay: OverlayNetworkManager::new(),
            overlay_address: None,
            // ...existing code...
        }
    }
    
    // ...existing code...
    
    /// Initialize the overlay network
    pub async fn initialize_overlay(&mut self, federation_id: Option<String>) -> Result<OverlayAddress> {
        let federation_id_ref = federation_id.as_deref();
        let address = self.overlay.initialize(&self.id.to_string(), federation_id_ref).await?;
        
        // Store the address
        self.overlay_address = Some(address.clone());
        
        info!("Node {} initialized overlay network with address: {:?}", self.id, address);
        Ok(address)
    }
    
    /// Connect to the overlay network using bootstrap nodes
    pub async fn connect_to_overlay(&mut self, bootstrap_addresses: Vec<OverlayAddress>) -> Result<()> {
        info!("Connecting to overlay network with {} bootstrap nodes", bootstrap_addresses.len());
        self.overlay.connect(&bootstrap_addresses).await?;
        info!("Node {} connected to overlay network", self.id);
        
        Ok(())
    }
    
    /// Send data through the overlay network
    pub async fn send_overlay_message(&self, destination: &OverlayAddress, data: Vec<u8>, 
                                      anonymity_required: bool) -> Result<()> {
        let options = OverlayOptions {
            anonymity_required,
            reliability_required: true,
            priority: MessagePriority::Normal,
        };
        
        self.overlay.send_data(destination, &data, &options).await?;
        debug!("Node {} sent message to {:?} through overlay", self.id, destination);
        
        Ok(())
    }
    
    /// Get the node's overlay address
    pub fn get_overlay_address(&self) -> Option<OverlayAddress> {
        self.overlay_address.clone()
    }
    
    // ...existing code...
}

// ...existing code...
