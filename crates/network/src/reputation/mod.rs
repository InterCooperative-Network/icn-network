mod enhanced_reputation;

/// Different contexts for reputation tracking
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum ReputationContext {
    /// General networking (connections, message processing)
    Networking,
    /// Consensus participation
    Consensus,
    /// Data validation and verification
    DataValidation,
    /// Resource sharing and provisioning
    ResourceSharing,
    /// Economic transactions
    Economic,
    /// Governance participation
    Governance,
    /// Custom context
    Custom(String),
}

pub use enhanced_reputation::{
    EnhancedReputationManager,
    EnhancedReputationConfig,
    ReputationMetrics,
    InteractionValue,
    ReputationEventHandler,
    ContributionMetricsProvider,
};

// For backward compatibility, re-export the enhanced manager as the standard manager
pub type ReputationManager = EnhancedReputationManager; 