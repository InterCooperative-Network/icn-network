//! Core node functionality for the Intercooperative Network
//!
//! This crate provides the fundamental components needed to run an ICN node,
//! including configuration, lifecycle management, and system integration.

use async_trait::async_trait;
use icn_common::{Error, Result};
use std::path::Path;
use std::sync::Arc;
use icn_common::{ComponentHealth, ComponentMetric, ComponentType, ICNComponent};
use std::any::Any;
use std::collections::HashMap;
use systems::capabilities::{CapabilityManager, HardwareProfile};
use crate::{
    error::Result as CrateResult,
    systems::{did_service::DidService, capabilities::CapabilityService},
    governance::GovernanceEngine,
    economics::EconomicEngine,
};

mod config;
mod state;
mod systems;

pub use config::{NodeConfig, NetworkMode};
pub use state::{NodeState, StateManager};
pub use systems::{DidServiceConfig};

/// The Node trait defines the core functionality of an ICN node
#[async_trait]
pub trait Node {
    /// Initialize the node with the given configuration
    async fn initialize(config: NodeConfig) -> Result<Self> where Self: Sized;
    
    /// Start the node's services
    async fn start(&mut self) -> Result<()>;
    
    /// Stop the node's services
    async fn stop(&mut self) -> Result<()>;
    
    /// Check if the node is currently running
    fn is_running(&self) -> bool;
    
    /// Get the current state of the node
    fn state(&self) -> NodeState;
    
    /// Get the node's configuration
    fn config(&self) -> &NodeConfig;
}

/// Basic ICN node implementation
pub struct IcnNode {
    /// Node configuration
    config: NodeConfig,
    
    /// Node state manager
    state_manager: Arc<StateManager>,
    
    /// DID service
    did_service: DidService,
    
    /// Node metrics
    metrics: HashMap<String, f64>,

    /// Capability manager
    capabilities: Option<CapabilityManager>,

    /// Capability service
    capability_service: CapabilityService,

    /// Governance engine
    governance_engine: GovernanceEngine,

    /// Economic engine
    economic_engine: EconomicEngine,
}

impl IcnNode {
    /// Create a new node with the specified configuration
    pub async fn new(config: NodeConfig) -> Result<Self> {
        Self::initialize(config).await
    }
    
    /// Create a new node from a configuration file
    pub async fn from_config_file(path: impl AsRef<Path>) -> Result<Self> {
        let config = NodeConfig::from_file(path)?;
        Self::new(config).await
    }
    
    /// Get the DID service if enabled
    pub fn did_service(&self) -> DidService {
        self.did_service.clone()
    }

    /// Get the capability manager if initialized
    pub fn capabilities(&self) -> Option<&CapabilityManager> {
        self.capabilities.as_ref()
    }
}

#[async_trait]
impl Node for IcnNode {
    async fn initialize(config: NodeConfig) -> Result<Self> {
        // Validate configuration
        config.validate()?;
        
        // Create state manager
        let state_manager = Arc::new(StateManager::new());
        
        // Initialize DID service
        let did_service = DidService::new(config.clone())?;

        // Initialize capabilities manager
        let mut capabilities = None;
        if config.capabilities.storage || config.capabilities.compute || config.capabilities.gateway {
            let hardware = HardwareProfile::detect();
            let mut manager = CapabilityManager::new(hardware, state_manager.clone());
            manager.initialize(&config.capabilities).await?;
            capabilities = Some(manager);
        }
        
        // Initialize capability service
        let capability_service = CapabilityService::new(config.clone())?;

        // Initialize governance and economic systems
        let governance_engine = GovernanceEngine::new();
        let economic_engine = EconomicEngine::new();
        
        Ok(Self {
            config,
            state_manager,
            did_service,
            metrics: HashMap::new(),
            capabilities,
            capability_service,
            governance_engine,
            economic_engine,
        })
    }
    
    async fn start(&mut self) -> Result<()> {
        if self.is_running() {
            return Err(Error::other("Node is already running"));
        }
        
        self.state_manager.transition(NodeState::Starting)?;
        
        // Start DID service
        self.did_service.start().await?;

        // Start capabilities if enabled
        if let Some(capabilities) = &mut self.capabilities {
            capabilities.start_all().await?;
        }

        // Initialize governance and economic systems
        self.initialize_governance().await?;
        self.initialize_economics().await?;
        
        self.state_manager.transition(NodeState::Running)?;
        Ok(())
    }
    
    async fn stop(&mut self) -> Result<()> {
        if !self.is_running() {
            return Err(Error::other("Node is not running"));
        }
        
        self.state_manager.transition(NodeState::Stopping)?;

        // Stop capabilities first
        if let Some(capabilities) = &mut self.capabilities {
            capabilities.stop_all().await?;
        }
        
        // Stop DID service
        self.did_service.stop().await?;
        
        self.state_manager.transition(NodeState::Stopped)?;
        Ok(())
    }
    
    fn is_running(&self) -> bool {
        matches!(self.state_manager.current_state(), NodeState::Running)
    }
    
    fn state(&self) -> NodeState {
        self.state_manager.current_state()
    }
    
    fn config(&self) -> &NodeConfig {
        &self.config
    }

    async fn initialize_governance(&mut self) -> Result<()> {
        // TODO: Load governance policies from storage
        // TODO: Initialize voting mechanisms
        // TODO: Set up policy execution engine
        Ok(())
    }

    async fn initialize_economics(&mut self) -> Result<()> {
        // TODO: Load economic assets from storage
        // TODO: Initialize treasury
        // TODO: Set up economic metrics tracking
        Ok(())
    }
}

#[async_trait]
impl ICNComponent for IcnNode {
    fn federation_id(&self) -> String {
        self.config.federation_id.clone()
    }

    fn component_type(&self) -> ComponentType {
        ComponentType::Identity
    }

    fn health_check(&self) -> ComponentHealth {
        let status = if self.is_running() {
            HealthStatus::Healthy
        } else {
            HealthStatus::Unhealthy
        };

        let mut metrics = self.metrics.clone();

        // Add capability metrics
        if let Some(capabilities) = &self.capabilities {
            let usage = capabilities.total_resource_usage();
            metrics.insert("cpu_percent".to_string(), usage.cpu_percent);
            metrics.insert("memory_mb".to_string(), usage.memory_mb as f64);
            metrics.insert("storage_gb".to_string(), usage.storage_gb as f64);
            metrics.insert("network_mbps".to_string(), usage.network_mbps);
        }

        ComponentHealth {
            status,
            message: Some(format!("Node is {}", self.state())),
            last_checked: chrono::Utc::now(),
            metrics,
        }
    }

    fn metrics(&self) -> Vec<ComponentMetric> {
        let mut metrics = Vec::new();
        
        // Add basic node metrics
        if let Some(uptime) = self.state_manager.uptime() {
            metrics.push(ComponentMetric {
                name: "uptime_seconds".to_string(),
                value: uptime.as_secs() as f64,
                labels: HashMap::new(),
                timestamp: chrono::Utc::now(),
            });
        }

        // Add component counts
        let summary = self.state_manager.status_summary();
        if let Some(running) = summary.get("components_running") {
            metrics.push(ComponentMetric {
                name: "components_running".to_string(),
                value: running.parse().unwrap_or(0.0),
                labels: HashMap::new(),
                timestamp: chrono::Utc::now(),
            });
        }

        metrics
    }

    fn shutdown(&self) -> Result<(), ShutdownError> {
        if self.is_running() {
            return Err(ShutdownError::StillRunning);
        }
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[tokio::test]
    async fn test_node_lifecycle() {
        let temp_dir = tempdir().unwrap();
        
        let mut config = NodeConfig::default();
        config.data_dir = temp_dir.path().to_string_lossy().to_string();
        config.federation_id = "test-fed-1".to_string();
        config.capabilities.storage = true;
        config.capabilities.max_storage_gb = Some(10);
        
        let mut node = IcnNode::new(config).await.unwrap();
        
        assert_eq!(node.state(), NodeState::Created);
        assert!(!node.is_running());
        assert_eq!(node.federation_id(), "test-fed-1");
        assert!(node.capabilities().is_some());
        
        node.start().await.unwrap();
        assert_eq!(node.state(), NodeState::Running);
        assert!(node.is_running());
        
        // Verify storage capability metrics
        let health = node.health_check();
        assert!(health.metrics.contains_key("storage_gb"));
        assert!(health.metrics.get("storage_gb").unwrap() <= &10.0);
        
        node.stop().await.unwrap();
        assert_eq!(node.state(), NodeState::Stopped);
        assert!(!node.is_running());
    }
    
    #[tokio::test]
    async fn test_node_from_config() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        
        let mut config = NodeConfig::default();
        config.data_dir = temp_dir.path().to_string_lossy().to_string();
        config.enable_identity = true;
        config.federation_id = "test-fed-2".to_string();
        
        config.save_to_file(&config_path).unwrap();
        
        let node = IcnNode::from_config_file(&config_path).await.unwrap();
        assert!(node.did_service().is_some());
        assert_eq!(node.federation_id(), "test-fed-2");
    }
}
