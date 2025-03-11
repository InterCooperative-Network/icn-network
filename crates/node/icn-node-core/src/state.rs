//! Node state management

use std::fmt;
use std::sync::{Arc, RwLock};
use icn_common::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Possible states of an ICN node
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

/// Component state information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentState {
    /// Name of the component
    pub name: String,
    
    /// Current state of the component
    pub state: String,
    
    /// If the component is enabled
    pub enabled: bool,
    
    /// Additional status details
    pub details: HashMap<String, String>,
    
    /// Last updated timestamp
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl ComponentState {
    /// Create a new component state
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            state: "initialized".to_string(),
            enabled: true,
            details: HashMap::new(),
            updated_at: chrono::Utc::now(),
        }
    }
    
    /// Update the component state
    pub fn update_state(&mut self, state: &str) -> &mut Self {
        self.state = state.to_string();
        self.updated_at = chrono::Utc::now();
        self
    }
    
    /// Add a detail to the component state
    pub fn with_detail(&mut self, key: &str, value: &str) -> &mut Self {
        self.details.insert(key.to_string(), value.to_string());
        self
    }
    
    /// Enable or disable the component
    pub fn set_enabled(&mut self, enabled: bool) -> &mut Self {
        self.enabled = enabled;
        self
    }
}

/// Node state transition callback
pub type StateTransitionCallback = Box<dyn Fn(NodeState, NodeState) -> Result<()> + Send + Sync>;

/// State manager for the ICN node
pub struct StateManager {
    /// Current node state
    state: RwLock<NodeState>,
    
    /// Component states
    component_states: RwLock<HashMap<String, ComponentState>>,
    
    /// Callbacks for state transitions
    transition_callbacks: RwLock<Vec<StateTransitionCallback>>,
    
    /// Performance metrics
    metrics: RwLock<HashMap<String, f64>>,
    
    /// State history
    history: RwLock<Vec<(NodeState, chrono::DateTime<chrono::Utc>)>>,
    
    /// Start time of the node
    start_time: Option<Instant>,
}

impl StateManager {
    /// Create a new state manager
    pub fn new() -> Self {
        let mut history = Vec::new();
        history.push((NodeState::Created, chrono::Utc::now()));
        
        Self {
            state: RwLock::new(NodeState::Created),
            component_states: RwLock::new(HashMap::new()),
            transition_callbacks: RwLock::new(Vec::new()),
            metrics: RwLock::new(HashMap::new()),
            history: RwLock::new(history),
            start_time: None,
        }
    }
    
    /// Get the current node state
    pub fn current_state(&self) -> NodeState {
        *self.state.read().unwrap()
    }
    
    /// Transition to a new state
    pub fn transition(&self, new_state: NodeState) -> Result<()> {
        let mut state = self.state.write().unwrap();
        let old_state = *state;
        
        // Execute callbacks
        for callback in self.transition_callbacks.read().unwrap().iter() {
            callback(old_state, new_state)?;
        }
        
        // Update state
        *state = new_state;
        
        // Update history
        self.history.write().unwrap().push((new_state, chrono::Utc::now()));
        
        // Update start time when transitioning to Running
        if new_state == NodeState::Running && old_state != NodeState::Running {
            let mut start_time = if let Some(ref mut start_time) = unsafe { &mut *((&self.start_time) as *const _ as *mut _) } {
                *start_time = Instant::now();
            } else {
                unsafe { *((&self.start_time) as *const _ as *mut _) = Some(Instant::now()); }
            };
        }
        
        tracing::info!("Node state transition: {} -> {}", old_state, new_state);
        Ok(())
    }
    
    /// Register a state transition callback
    pub fn register_transition_callback(&self, callback: StateTransitionCallback) {
        self.transition_callbacks.write().unwrap().push(callback);
    }
    
    /// Register a component and initialize its state
    pub fn register_component(&self, name: &str) -> Result<()> {
        let mut components = self.component_states.write().unwrap();
        if components.contains_key(name) {
            return Err(Error::validation(format!("Component {} already registered", name)));
        }
        
        components.insert(name.to_string(), ComponentState::new(name));
        Ok(())
    }
    
    /// Update a component's state
    pub fn update_component(&self, name: &str, state: &str) -> Result<()> {
        let mut components = self.component_states.write().unwrap();
        if let Some(component) = components.get_mut(name) {
            component.update_state(state);
            Ok(())
        } else {
            Err(Error::not_found(format!("Component {} not found", name)))
        }
    }
    
    /// Get a component's state
    pub fn get_component(&self, name: &str) -> Result<ComponentState> {
        let components = self.component_states.read().unwrap();
        if let Some(component) = components.get(name) {
            Ok(component.clone())
        } else {
            Err(Error::not_found(format!("Component {} not found", name)))
        }
    }
    
    /// Get all component states
    pub fn get_all_components(&self) -> Vec<ComponentState> {
        let components = self.component_states.read().unwrap();
        components.values().cloned().collect()
    }
    
    /// Record a metric
    pub fn record_metric(&self, name: &str, value: f64) {
        let mut metrics = self.metrics.write().unwrap();
        metrics.insert(name.to_string(), value);
    }
    
    /// Get a metric
    pub fn get_metric(&self, name: &str) -> Option<f64> {
        let metrics = self.metrics.read().unwrap();
        metrics.get(name).copied()
    }
    
    /// Get node uptime
    pub fn uptime(&self) -> Option<Duration> {
        self.start_time.map(|t| t.elapsed())
    }
    
    /// Get state history
    pub fn history(&self) -> Vec<(NodeState, chrono::DateTime<chrono::Utc>)> {
        self.history.read().unwrap().clone()
    }
    
    /// Get node status summary
    pub fn status_summary(&self) -> HashMap<String, String> {
        let mut summary = HashMap::new();
        
        // Basic state info
        summary.insert("state".to_string(), self.current_state().to_string());
        
        // Uptime if running
        if let Some(uptime) = self.uptime() {
            let secs = uptime.as_secs();
            let hrs = secs / 3600;
            let mins = (secs % 3600) / 60;
            let secs = secs % 60;
            summary.insert("uptime".to_string(), format!("{}h {}m {}s", hrs, mins, secs));
        }
        
        // Component count
        let components = self.component_states.read().unwrap();
        summary.insert("component_count".to_string(), components.len().to_string());
        
        // Count by state
        let mut running = 0;
        let mut error = 0;
        for component in components.values() {
            match component.state.as_str() {
                "running" => running += 1,
                "error" => error += 1,
                _ => {}
            }
        }
        
        summary.insert("components_running".to_string(), running.to_string());
        summary.insert("components_error".to_string(), error.to_string());
        
        summary
    }
}

impl Default for StateManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a shared state manager
pub fn create_state_manager() -> Arc<StateManager> {
    Arc::new(StateManager::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_state_transitions() {
        let manager = StateManager::new();
        
        assert_eq!(manager.current_state(), NodeState::Created);
        
        // Test simple transition
        manager.transition(NodeState::Initialized).unwrap();
        assert_eq!(manager.current_state(), NodeState::Initialized);
        
        // Test history
        let history = manager.history();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].0, NodeState::Created);
        assert_eq!(history[1].0, NodeState::Initialized);
    }
    
    #[test]
    fn test_component_states() {
        let manager = StateManager::new();
        
        // Register components
        manager.register_component("network").unwrap();
        manager.register_component("identity").unwrap();
        
        // Update component state
        manager.update_component("network", "running").unwrap();
        
        // Get component state
        let network = manager.get_component("network").unwrap();
        assert_eq!(network.state, "running");
        
        // Get all components
        let components = manager.get_all_components();
        assert_eq!(components.len(), 2);
        
        // Test non-existent component
        assert!(manager.get_component("nonexistent").is_err());
    }
}
