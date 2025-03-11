//! Common utilities and types for the Intercooperative Network
//! 
//! This crate provides shared utilities, error types, and common functionality
//! used throughout the ICN project.

use std::any::Any;
use async_trait::async_trait;
use serde::{Serialize, Deserialize};

pub mod error;
pub mod config;
pub mod types;
pub mod utils;

pub use error::{Error, Result, ShutdownError};
pub use types::{ComponentHealth, ComponentMetric, ComponentType, Version, HealthStatus};

/// Re-export common traits
pub trait Identifiable {
    /// Get the unique identifier for this entity
    fn id(&self) -> &str;
}

/// Core trait that all system components must implement
#[async_trait]
pub trait ICNComponent: Send + Sync {
    /// Get the federation ID this component belongs to
    fn federation_id(&self) -> String;

    /// Get the type of this component
    fn component_type(&self) -> ComponentType;

    /// Perform a health check
    fn health_check(&self) -> ComponentHealth;

    /// Get current metrics
    fn metrics(&self) -> Vec<ComponentMetric>;

    /// Shut down the component
    fn shutdown(&self) -> Result<(), ShutdownError>;

    /// Cast to Any for dynamic dispatch
    fn as_any(&self) -> &dyn Any;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComponentType {
    Identity,
    Governance,
    Economic,
    Resource,
    Consensus,
    Storage, 
    Network,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    pub status: HealthStatus,
    pub message: Option<String>,
    pub last_checked: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentMetric {
    pub name: String,
    pub value: f64,
    pub labels: std::collections::HashMap<String, String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, thiserror::Error)]
pub enum ComponentError {
    #[error("Initialization failed: {0}")]
    InitializationFailed(String),
    
    #[error("Operation failed: {0}")]
    OperationFailed(String),
    
    #[error("Invalid state: {0}")]
    InvalidState(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use chrono::Utc;

    #[test]
    fn test_identifiable_trait() {
        struct TestEntity(&'static str);
        
        impl Identifiable for TestEntity {
            fn id(&self) -> &str {
                self.0
            }
        }
        
        let entity = TestEntity("test-id-1");
        assert_eq!(entity.id(), "test-id-1");
    }

    struct TestComponent {
        federation_id: String,
    }

    #[async_trait]
    impl ICNComponent for TestComponent {
        fn federation_id(&self) -> String {
            self.federation_id.clone()
        }

        fn component_type(&self) -> ComponentType {
            ComponentType::Identity
        }

        fn health_check(&self) -> ComponentHealth {
            ComponentHealth {
                status: HealthStatus::Healthy,
                message: None,
                last_checked: Utc::now(),
                metrics: HashMap::new(),
            }
        }

        fn metrics(&self) -> Vec<ComponentMetric> {
            vec![]
        }

        fn shutdown(&self) -> Result<(), ShutdownError> {
            Ok(())
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    #[test]
    fn test_component_trait() {
        let component = TestComponent {
            federation_id: "test-fed-1".to_string(),
        };

        assert_eq!(component.federation_id(), "test-fed-1");
        assert!(matches!(component.component_type(), ComponentType::Identity));
        assert!(matches!(component.health_check().status, HealthStatus::Healthy));
        assert!(component.shutdown().is_ok());
    }
}
