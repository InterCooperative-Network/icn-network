//! Common utilities and types for the Intercooperative Network
//! 
//! This crate provides shared utilities, error types, and common functionality
//! used throughout the ICN project.

pub mod error;
pub mod config;
pub mod types;
pub mod utils;

pub use error::{Error, Result};

/// Re-export common traits
pub trait Identifiable {
    /// Get the unique identifier for this entity
    fn id(&self) -> &str;
}

#[cfg(test)]
mod tests {
    use super::*;
    
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
}
