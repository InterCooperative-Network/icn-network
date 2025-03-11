//! Intercooperative Network (ICN) 
//! 
//! A modular, component-based system for cooperative networks
//! with emphasis on identity, governance, and economic systems.

/// Module version information
pub mod version {
    /// The current version of the ICN library
    pub const VERSION: &str = env!("CARGO_PKG_VERSION");
}

/// Re-export core components for easy access
pub mod core {
    pub use icn_common as common;
    pub use icn_crypto as crypto;
    pub use icn_data_structures as data;
    pub use icn_serialization as serialization;
}

/// Re-export system components
pub mod systems {
    pub use icn_identity_system as identity;
    pub use icn_governance_system as governance;
    pub use icn_economic_system as economic;
}

/// Node implementation components
pub mod node {
    pub use icn_node_core as core;
}

#[cfg(test)]
mod tests {
    #[test]
    fn version_is_available() {
        assert!(!super::version::VERSION.is_empty());
    }
}
