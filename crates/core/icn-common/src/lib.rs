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
    fn shutdown(&self) -> Result<()>;

    /// Cast to Any for dynamic dispatch
    fn as_any(&self) -> &dyn Any;
}

#[cfg(test)]
mod tests {
    use super::*;
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
            }
        }

        fn metrics(&self) -> Vec<ComponentMetric> {
            vec![]
        }

        fn shutdown(&self) -> Result<()> {
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
