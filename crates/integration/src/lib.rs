//! Integration modules for connecting different ICN subsystems
//! 
//! This crate provides integration between various ICN subsystems,
//! enabling them to work together in a cohesive manner.

pub mod overlay_integration;

pub use overlay_integration::{
    OverlayIntegration, OverlayMessage, EconomicMessage, GovernanceMessage, 
    ResourceMessage, NetworkMessage, TransactionStatus, VotingResults,
    ProposalStatus, ResourceRequestStatus, NodeCapability, ResourceAvailability
};
