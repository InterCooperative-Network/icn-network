use std::sync::Arc;
use std::error::Error;

use icn_core::di::DependencyContainer;
use icn_core::interfaces::identity::IdentityProvider;
use icn_core::interfaces::storage::StorageProvider;
use icn_core::interfaces::reputation::ReputationProvider;
use icn_core::interfaces::economic::EconomicProvider;

use crate::ledger::{MutualCreditSystem, MutualCreditConfig};

/// Factory for creating MutualCreditSystem instances
pub struct MutualCreditFactory;

impl MutualCreditFactory {
    /// Create a new MutualCreditSystem from a dependency container
    pub fn create_from_container(
        container: &DependencyContainer,
        config: MutualCreditConfig,
    ) -> Result<Arc<dyn EconomicProvider>, Box<dyn Error + Send + Sync>> {
        // Resolve required dependencies
        let identity_provider = container
            .resolve::<dyn IdentityProvider>()
            .ok_or_else(|| "Identity provider not found in container".to_string())?;
        
        let storage_provider = container
            .resolve::<dyn StorageProvider>()
            .ok_or_else(|| "Storage provider not found in container".to_string())?;
        
        let reputation_provider = container
            .resolve::<dyn ReputationProvider>()
            .ok_or_else(|| "Reputation provider not found in container".to_string())?;
        
        // Create the MutualCreditSystem
        let system = MutualCreditSystem::new(
            config,
            identity_provider,
            storage_provider,
            reputation_provider,
        );
        
        // Return as a boxed EconomicProvider trait object
        Ok(Arc::new(system))
    }
} 