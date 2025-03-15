//! Utility functions for ICN
//!
//! This module provides common utility functions and types used across the
//! InterCooperative Network codebase.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::str::FromStr;
use serde::{Serialize, Deserialize};
use thiserror::Error;
use tokio::fs;

/// Error types for utility operations
#[derive(Error, Debug)]
pub enum UtilError {
    /// IO error
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    /// Parsing error
    #[error("Parsing error: {0}")]
    ParseError(String),
    
    /// Invalid value
    #[error("Invalid value: {0}")]
    InvalidValue(String),
}

/// Result type for utility operations
pub type UtilResult<T> = Result<T, UtilError>;

/// Get the current timestamp in milliseconds
pub fn timestamp_ms() -> u64 {
    let start = SystemTime::now();
    let since_epoch = start.duration_since(UNIX_EPOCH).unwrap_or(Duration::from_secs(0));
    (since_epoch.as_secs() * 1000) + (since_epoch.subsec_nanos() as u64 / 1_000_000)
}

/// Get the current timestamp in seconds
pub fn timestamp_secs() -> u64 {
    let start = SystemTime::now();
    let since_epoch = start.duration_since(UNIX_EPOCH).unwrap_or(Duration::from_secs(0));
    since_epoch.as_secs()
}

/// Check if a path exists
pub async fn path_exists<P: AsRef<Path>>(path: P) -> bool {
    fs::metadata(path).await.is_ok()
}

/// Create all parent directories for a path
pub async fn create_parent_dirs<P: AsRef<Path>>(path: P) -> UtilResult<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).await?;
        }
    }
    Ok(())
}

/// Parse a string to a specific type
pub fn parse_string<T: FromStr>(value: &str) -> UtilResult<T>
where
    T::Err: std::fmt::Display,
{
    value.parse::<T>()
        .map_err(|e| UtilError::ParseError(format!("Failed to parse value: {}", e)))
}

/// A unit of resource measurement
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ResourceUnit {
    /// Count (no units)
    Count,
    /// Bytes
    Bytes,
    /// Seconds
    Seconds,
    /// Watts
    Watts,
    /// Currency units
    Currency,
    /// Custom unit (with a name)
    Custom(String),
}

impl Default for ResourceUnit {
    fn default() -> Self {
        Self::Count
    }
}

impl ToString for ResourceUnit {
    fn to_string(&self) -> String {
        match self {
            Self::Count => "count".to_string(),
            Self::Bytes => "bytes".to_string(),
            Self::Seconds => "seconds".to_string(),
            Self::Watts => "watts".to_string(),
            Self::Currency => "currency".to_string(),
            Self::Custom(name) => name.clone(),
        }
    }
}

impl FromStr for ResourceUnit {
    type Err = UtilError;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let lower = s.to_lowercase();
        match lower.as_str() {
            "count" => Ok(Self::Count),
            "bytes" | "byte" | "b" => Ok(Self::Bytes),
            "seconds" | "second" | "s" | "sec" | "secs" => Ok(Self::Seconds),
            "watts" | "watt" | "w" => Ok(Self::Watts),
            "currency" | "curr" | "c" => Ok(Self::Currency),
            _ => Ok(Self::Custom(s.to_string())),
        }
    }
}

/// A quantity of a resource
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResourceQuantity {
    /// The amount
    pub amount: f64,
    /// The unit
    pub unit: ResourceUnit,
}

impl ResourceQuantity {
    /// Create a new resource quantity
    pub fn new(amount: f64, unit: ResourceUnit) -> Self {
        Self { amount, unit }
    }
    
    /// Add another quantity
    pub fn add(&self, other: &Self) -> UtilResult<Self> {
        if self.unit != other.unit {
            return Err(UtilError::InvalidValue(
                format!("Cannot add quantities with different units: {} and {}", 
                    self.unit.to_string(), other.unit.to_string())
            ));
        }
        
        Ok(Self {
            amount: self.amount + other.amount,
            unit: self.unit.clone(),
        })
    }
    
    /// Subtract another quantity
    pub fn subtract(&self, other: &Self) -> UtilResult<Self> {
        if self.unit != other.unit {
            return Err(UtilError::InvalidValue(
                format!("Cannot subtract quantities with different units: {} and {}", 
                    self.unit.to_string(), other.unit.to_string())
            ));
        }
        
        Ok(Self {
            amount: self.amount - other.amount,
            unit: self.unit.clone(),
        })
    }
    
    /// Multiply by a scalar
    pub fn multiply(&self, scalar: f64) -> Self {
        Self {
            amount: self.amount * scalar,
            unit: self.unit.clone(),
        }
    }
    
    /// Divide by a scalar
    pub fn divide(&self, scalar: f64) -> UtilResult<Self> {
        if scalar == 0.0 {
            return Err(UtilError::InvalidValue("Cannot divide by zero".to_string()));
        }
        
        Ok(Self {
            amount: self.amount / scalar,
            unit: self.unit.clone(),
        })
    }
}

pub mod validation;
pub mod serialization; 