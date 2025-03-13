//! Integration modules for connecting different ICN subsystems

mod overlay_integration;

pub use overlay_integration::{
    OverlayIntegration, OverlayMessage, EconomicMessage, GovernanceMessage, 
    ResourceMessage, NetworkMessage
};
