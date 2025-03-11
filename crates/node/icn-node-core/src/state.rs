//! Node state management

use std::fmt;

/// Possible states of an ICN node
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeState {
    /// Node is created but not initialized
    Created,
    /// Node is initialized but not started
    Initialized,
    /// Node is starting up
    Starting,
    /// Node is running
    Running,
    /// Node is stopping
    Stopping,
    /// Node is stopped
    Stopped,
    /// Node is in error state
    Error,
}

impl fmt::Display for NodeState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NodeState::Created => write!(f, "Created"),
            NodeState::Initialized => write!(f, "Initialized"),
            NodeState::Starting => write!(f, "Starting"),
            NodeState::Running => write!(f, "Running"),
            NodeState::Stopping => write!(f, "Stopping"),
            NodeState::Stopped => write!(f, "Stopped"),
            NodeState::Error => write!(f, "Error"),
        }
    }
}

impl Default for NodeState {
    fn default() -> Self {
        NodeState::Created
    }
}
