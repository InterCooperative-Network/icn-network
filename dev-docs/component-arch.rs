/// Core trait that all system components must implement
pub trait ICNComponent {
    fn federation_id(&self) -> FederationId;
    fn component_type(&self) -> ComponentType;
    fn health_check(&self) -> ComponentHealth;
    fn metrics(&self) -> Vec<Metric>;
    fn shutdown(&self) -> Result<(), ShutdownError>;
}

/// All systems derive from a common base with shared functionality
pub struct ICNSystem<T: ICNComponent> {
    component: T,
    security_layer: SecurityLayer,
    connection_manager: ConnectionManager,
    state_tracker: StateTracker,
    federation_context: FederationContext,
}

/// Federation context for cross-federation operations
pub struct FederationContext {
    federation_id: FederationId,
    trust_level: TrustLevel,
    cross_federation_policy: FederationPolicy,
    discovery_service: DiscoveryService,
}

/// Component types available in the system
pub enum ComponentType {
    Identity,
    Governance,
    Economic,
    Resource,
    Consensus,
    Storage,
    Network,
    // Additional component types
}

/// Health status for component monitoring
pub struct ComponentHealth {
    status: HealthStatus,
    details: String,
    last_checked: Timestamp,
    metrics: HashMap<String, f64>,
}

/// Status of component health
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}
