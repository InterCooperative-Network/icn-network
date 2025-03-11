//! Common types used throughout the ICN project

use serde::{Deserialize, Serialize};
use std::fmt;
use std::collections::HashMap;
use chrono::{DateTime, Utc};

/// A unique identifier type
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Identifier(String);

impl Identifier {
    /// Create a new identifier
    pub fn new<S: Into<String>>(id: S) -> Self {
        Self(id.into())
    }
    
    /// Get the identifier as a string reference
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Identifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for Identifier {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for Identifier {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Type of component in the ICN system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComponentType {
    /// Identity management component
    Identity,
    /// Governance component
    Governance,
    /// Economic component
    Economic,
    /// Resource management component
    Resource,
    /// Consensus component
    Consensus,
    /// Storage component
    Storage,
    /// Network component
    Network,
}

/// Health status of a component
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    /// Component is healthy and functioning normally
    Healthy,
    /// Component is functioning but with reduced capabilities
    Degraded,
    /// Component is not functioning
    Unhealthy,
    /// Component health status cannot be determined
    Unknown,
}

/// Health information for a component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    /// Current health status
    pub status: HealthStatus,
    /// Optional message providing more details about the health status
    pub message: Option<String>,
    /// Timestamp of when the health check was performed
    pub last_checked: DateTime<Utc>,
}

/// Metric information for a component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentMetric {
    /// Name of the metric
    pub name: String,
    /// Numeric value of the metric
    pub value: f64,
    /// Additional labels/tags for the metric
    pub labels: HashMap<String, String>,
    /// Timestamp when the metric was recorded
    pub timestamp: DateTime<Utc>,
}

/// Version information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Version {
    /// Major version number
    pub major: u32,
    /// Minor version number
    pub minor: u32,
    /// Patch version number
    pub patch: u32,
}

impl Version {
    /// Create a new version
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self { major, minor, patch }
    }
    
    /// Parse a version from a string like "1.2.3"
    pub fn parse(version_str: &str) -> Option<Self> {
        let parts: Vec<&str> = version_str.split('.').collect();
        if parts.len() != 3 {
            return None;
        }
        
        let major = parts[0].parse().ok()?;
        let minor = parts[1].parse().ok()?;
        let patch = parts[2].parse().ok()?;
        
        Some(Self::new(major, minor, patch))
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}
