//! Dependency Injection module for managing component dependencies
//!
//! This module provides a simple dependency injection container that allows
//! for registration and resolution of components based on their interface types.

mod container;

pub use container::DependencyContainer; 