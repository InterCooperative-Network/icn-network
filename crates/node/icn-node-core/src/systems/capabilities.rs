use std::sync::Arc;
use async_trait::async_trait;
use icn_common::{ComponentHealth, ComponentMetric, ComponentType, ICNComponent, Result, Error, ShutdownError};
use crate::state::StateManager;

/// Hardware profile for determining available capabilities
pub struct HardwareProfile {
    /// Number of CPU cores
    pub cpu_cores: u32,
    
    /// Available memory in MB
    pub memory_mb: u64,
    
    /// Available storage in GB
    pub storage_gb: u64,
    
    /// Network bandwidth in Mbps
    pub network_mbps: u32,
    
    /// Whether the node is on a stable connection
    pub is_stable: bool,
    
    /// Whether hardware crypto acceleration is available
    pub has_crypto_acceleration: bool,
}

impl HardwareProfile {
    /// Detect hardware capabilities of the current system
    pub fn detect() -> Self {
        // TODO: Implement actual hardware detection
        Self {
            cpu_cores: num_cpus::get() as u32,
            memory_mb: sys_info::mem_info().map(|m| m.total / 1024).unwrap_or(1024),
            storage_gb: 100, // TODO: Implement storage detection
            network_mbps: 100, // TODO: Implement bandwidth detection
            is_stable: true,
            has_crypto_acceleration: false,
        }
    }

    /// Check if the system meets minimum requirements
    pub fn meets_minimum_requirements(&self) -> bool {
        self.cpu_cores >= 1 && 
        self.memory_mb >= 512 &&
        self.storage_gb >= 1
    }
}

/// Base trait for node capabilities
#[async_trait]
pub trait NodeCapability: ICNComponent {
    /// Initialize the capability
    async fn initialize(&mut self) -> Result<()>;
    
    /// Start the capability
    async fn start(&mut self) -> Result<()>;
    
    /// Stop the capability
    async fn stop(&mut self) -> Result<()>;
    
    /// Get resource usage metrics
    fn resource_usage(&self) -> ResourceUsage;
}

/// Resource usage metrics
#[derive(Debug, Clone)]
pub struct ResourceUsage {
    /// CPU usage percentage (0-100)
    pub cpu_percent: f64,
    
    /// Memory usage in MB
    pub memory_mb: u64,
    
    /// Storage usage in GB
    pub storage_gb: u64,
    
    /// Network bandwidth usage in Mbps
    pub network_mbps: f64,
}

/// Manager for node capabilities
pub struct CapabilityManager {
    hardware: HardwareProfile,
    state_manager: Arc<StateManager>,
    storage: Option<StorageCapability>,
    compute: Option<ComputeCapability>,
    gateway: Option<GatewayCapability>,
}

impl CapabilityManager {
    /// Create a new capability manager
    pub fn new(hardware: HardwareProfile, state_manager: Arc<StateManager>) -> Self {
        Self {
            hardware,
            state_manager,
            storage: None,
            compute: None,
            gateway: None,
        }
    }

    /// Initialize capabilities based on hardware profile and configuration
    pub async fn initialize(&mut self, config: &crate::config::NodeCapabilitiesConfig) -> Result<()> {
        // Validate hardware meets minimum requirements
        if !self.hardware.meets_minimum_requirements() {
            return Err(Error::validation("System does not meet minimum hardware requirements"));
        }

        // Initialize storage if enabled and requirements met
        if config.storage && 
           config.max_storage_gb.unwrap_or(0) <= self.hardware.storage_gb {
            let storage = StorageCapability::new(
                config.max_storage_gb.unwrap_or(self.hardware.storage_gb),
                self.state_manager.clone(),
            );
            storage.initialize().await?;
            self.storage = Some(storage);
        }

        // Initialize compute if enabled and requirements met
        if config.compute && 
           config.max_cpu_cores.unwrap_or(0) as u32 <= self.hardware.cpu_cores &&
           config.max_memory_mb.unwrap_or(0) <= self.hardware.memory_mb {
            let compute = ComputeCapability::new(
                config.max_cpu_cores.unwrap_or(self.hardware.cpu_cores),
                config.max_memory_mb.unwrap_or(self.hardware.memory_mb),
                self.state_manager.clone(),
            );
            compute.initialize().await?;
            self.compute = Some(compute);
        }

        // Initialize gateway if enabled and requirements met
        if config.gateway && self.hardware.is_stable {
            let gateway = GatewayCapability::new(self.state_manager.clone());
            gateway.initialize().await?;
            self.gateway = Some(gateway);
        }

        Ok(())
    }

    /// Start all enabled capabilities
    pub async fn start_all(&mut self) -> Result<()> {
        if let Some(storage) = &mut self.storage {
            storage.start().await?;
        }
        if let Some(compute) = &mut self.compute {
            compute.start().await?;
        }
        if let Some(gateway) = &mut self.gateway {
            gateway.start().await?;
        }
        Ok(())
    }

    /// Stop all enabled capabilities
    pub async fn stop_all(&mut self) -> Result<()> {
        if let Some(gateway) = &mut self.gateway {
            gateway.stop().await?;
        }
        if let Some(compute) = &mut self.compute {
            compute.stop().await?;
        }
        if let Some(storage) = &mut self.storage {
            storage.stop().await?;
        }
        Ok(())
    }

    /// Get total resource usage across all capabilities
    pub fn total_resource_usage(&self) -> ResourceUsage {
        let mut total = ResourceUsage {
            cpu_percent: 0.0,
            memory_mb: 0,
            storage_gb: 0,
            network_mbps: 0.0,
        };

        if let Some(storage) = &self.storage {
            let usage = storage.resource_usage();
            total.storage_gb += usage.storage_gb;
        }
        if let Some(compute) = &self.compute {
            let usage = compute.resource_usage();
            total.cpu_percent += usage.cpu_percent;
            total.memory_mb += usage.memory_mb;
        }
        if let Some(gateway) = &self.gateway {
            let usage = gateway.resource_usage();
            total.network_mbps += usage.network_mbps;
        }

        total
    }
}

// Placeholder capability implementations that will be fully implemented in their respective crates
pub struct StorageCapability {
    max_storage_gb: u64,
    state_manager: Arc<StateManager>,
}

impl StorageCapability {
    pub fn new(max_storage_gb: u64, state_manager: Arc<StateManager>) -> Self {
        Self {
            max_storage_gb,
            state_manager,
        }
    }
}

#[async_trait]
impl NodeCapability for StorageCapability {
    async fn initialize(&mut self) -> Result<()> {
        self.state_manager.register_component("storage")?;
        Ok(())
    }

    async fn start(&mut self) -> Result<()> {
        self.state_manager.update_component("storage", "running")?;
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        self.state_manager.update_component("storage", "stopped")?;
        Ok(())
    }

    fn resource_usage(&self) -> ResourceUsage {
        // TODO: Implement actual storage usage monitoring
        ResourceUsage {
            cpu_percent: 0.0,
            memory_mb: 0,
            storage_gb: self.max_storage_gb,
            network_mbps: 0.0,
        }
    }
}

#[async_trait]
impl ICNComponent for StorageCapability {
    fn federation_id(&self) -> String {
        // Use node's federation ID from state manager when available
        "default".to_string()
    }

    fn component_type(&self) -> ComponentType {
        ComponentType::Storage
    }

    fn health_check(&self) -> ComponentHealth {
        ComponentHealth {
            status: HealthStatus::Healthy, // TODO: Implement actual health check
            message: None,
            last_checked: chrono::Utc::now(),
            metrics: HashMap::new(),
        }
    }

    fn metrics(&self) -> Vec<ComponentMetric> {
        let usage = self.resource_usage();
        vec![
            ComponentMetric {
                name: "storage_used_gb".to_string(),
                value: usage.storage_gb as f64,
                labels: HashMap::new(),
                timestamp: chrono::Utc::now(),
            }
        ]
    }

    fn shutdown(&self) -> Result<(), ShutdownError> {
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub struct ComputeCapability {
    max_cpu_cores: u32,
    max_memory_mb: u64,
    state_manager: Arc<StateManager>,
}

impl ComputeCapability {
    pub fn new(max_cpu_cores: u32, max_memory_mb: u64, state_manager: Arc<StateManager>) -> Self {
        Self {
            max_cpu_cores,
            max_memory_mb,
            state_manager,
        }
    }
}

#[async_trait]
impl NodeCapability for ComputeCapability {
    async fn initialize(&mut self) -> Result<()> {
        self.state_manager.register_component("compute")?;
        Ok(())
    }

    async fn start(&mut self) -> Result<()> {
        self.state_manager.update_component("compute", "running")?;
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        self.state_manager.update_component("compute", "stopped")?;
        Ok(())
    }

    fn resource_usage(&self) -> ResourceUsage {
        // TODO: Implement actual compute resource monitoring
        ResourceUsage {
            cpu_percent: 0.0,
            memory_mb: 0,
            storage_gb: 0,
            network_mbps: 0.0,
        }
    }
}

#[async_trait]
impl ICNComponent for ComputeCapability {
    fn federation_id(&self) -> String {
        "default".to_string()
    }

    fn component_type(&self) -> ComponentType {
        ComponentType::Resource
    }

    fn health_check(&self) -> ComponentHealth {
        ComponentHealth {
            status: HealthStatus::Healthy,
            message: None,
            last_checked: chrono::Utc::now(),
            metrics: HashMap::new(),
        }
    }

    fn metrics(&self) -> Vec<ComponentMetric> {
        let usage = self.resource_usage();
        vec![
            ComponentMetric {
                name: "cpu_usage_percent".to_string(),
                value: usage.cpu_percent,
                labels: HashMap::new(),
                timestamp: chrono::Utc::now(),
            },
            ComponentMetric {
                name: "memory_used_mb".to_string(),
                value: usage.memory_mb as f64,
                labels: HashMap::new(),
                timestamp: chrono::Utc::now(),
            }
        ]
    }

    fn shutdown(&self) -> Result<(), ShutdownError> {
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub struct GatewayCapability {
    state_manager: Arc<StateManager>,
}

impl GatewayCapability {
    pub fn new(state_manager: Arc<StateManager>) -> Self {
        Self {
            state_manager,
        }
    }
}

#[async_trait]
impl NodeCapability for GatewayCapability {
    async fn initialize(&mut self) -> Result<()> {
        self.state_manager.register_component("gateway")?;
        Ok(())
    }

    async fn start(&mut self) -> Result<()> {
        self.state_manager.update_component("gateway", "running")?;
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        self.state_manager.update_component("gateway", "stopped")?;
        Ok(())
    }

    fn resource_usage(&self) -> ResourceUsage {
        ResourceUsage {
            cpu_percent: 0.0,
            memory_mb: 0,
            storage_gb: 0,
            network_mbps: 0.0, // TODO: Implement network usage monitoring
        }
    }
}

#[async_trait]
impl ICNComponent for GatewayCapability {
    fn federation_id(&self) -> String {
        "default".to_string()
    }

    fn component_type(&self) -> ComponentType {
        ComponentType::Network
    }

    fn health_check(&self) -> ComponentHealth {
        ComponentHealth {
            status: HealthStatus::Healthy,
            message: None,
            last_checked: chrono::Utc::now(),
            metrics: HashMap::new(),
        }
    }

    fn metrics(&self) -> Vec<ComponentMetric> {
        let usage = self.resource_usage();
        vec![
            ComponentMetric {
                name: "network_bandwidth_mbps".to_string(),
                value: usage.network_mbps,
                labels: HashMap::new(),
                timestamp: chrono::Utc::now(),
            }
        ]
    }

    fn shutdown(&self) -> Result<(), ShutdownError> {
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_capability_manager() {
        let state_manager = Arc::new(StateManager::new());
        let hardware = HardwareProfile {
            cpu_cores: 4,
            memory_mb: 8192,
            storage_gb: 100,
            network_mbps: 1000,
            is_stable: true,
            has_crypto_acceleration: false,
        };

        let mut manager = CapabilityManager::new(hardware, state_manager);

        let config = crate::config::NodeCapabilitiesConfig {
            storage: true,
            compute: true,
            gateway: true,
            max_storage_gb: Some(50),
            max_cpu_cores: Some(2),
            max_memory_mb: Some(4096),
        };

        manager.initialize(&config).await.unwrap();
        manager.start_all().await.unwrap();

        let usage = manager.total_resource_usage();
        assert!(usage.storage_gb <= 50);
        assert!(usage.cpu_percent <= 100.0);
        assert!(usage.memory_mb <= 4096);

        manager.stop_all().await.unwrap();
    }
}