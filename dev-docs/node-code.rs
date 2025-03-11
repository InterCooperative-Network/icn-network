pub struct ICNNode {
    // Core components that every node must have
    identity: IdentityComponent,
    networking: NetworkingComponent,
    consensus: ConsensusComponent,
    
    // Optional capabilities that can be enabled/disabled
    capabilities: NodeCapabilities,
    
    // Hardware profile to adapt behavior
    hardware_profile: HardwareProfile,
}

pub struct NodeCapabilities {
    governance: Option<GovernanceCapability>,
    storage: Option<StorageCapability>,
    compute: Option<ComputeCapability>,
    gateway: Option<GatewayCapability>,
    // Additional optional capabilities
}

pub struct HardwareProfile {
    cpu_cores: u32,
    memory_mb: u64,
    storage_gb: u64,
    network_mbps: u32,
    is_stable: bool,
    has_crypto_acceleration: bool,
}

impl ICNNode {
    // Create a new node with capabilities based on hardware
    pub fn new(hardware: HardwareProfile) -> Self {
        // Create base components
        let identity = IdentityComponent::new();
        let networking = NetworkingComponent::new();
        let consensus = ConsensusComponent::new();
        
        // Determine capabilities based on hardware
        let capabilities = NodeCapabilities {
            governance: if hardware.cpu_cores >= 2 { 
                Some(GovernanceCapability::new())
            } else { 
                None 
            },
            
            storage: if hardware.storage_gb >= 10 { 
                Some(StorageCapability::new(hardware.storage_gb))
            } else { 
                None 
            },
            
            compute: if hardware.cpu_cores >= 4 && hardware.memory_mb >= 4096 { 
                Some(ComputeCapability::new())
            } else { 
                None 
            },
            
            gateway: if hardware.network_mbps >= 50 && hardware.is_stable { 
                Some(GatewayCapability::new())
            } else { 
                None 
            },
        };
        
        ICNNode {
            identity,
            networking,
            consensus,
            capabilities,
            hardware_profile: hardware,
        }
    }
    
    // Start the node and its enabled capabilities
    pub fn start(&mut self) -> Result<(), NodeError> {
        // Start core components
        self.identity.start()?;
        self.networking.start()?;
        self.consensus.start()?;
        
        // Start optional capabilities if enabled
        if let Some(ref mut gov) = self.capabilities.governance {
            gov.start()?;
        }
        
        if let Some(ref mut storage) = self.capabilities.storage {
            storage.start()?;
        }
        
        if let Some(ref mut compute) = self.capabilities.compute {
            compute.start()?;
        }
        
        if let Some(ref mut gateway) = self.capabilities.gateway {
            gateway.start()?;
        }
        
        Ok(())
    }
}
