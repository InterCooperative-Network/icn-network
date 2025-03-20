use std::sync::Arc;
use async_trait::async_trait;
use icn_core::di::DependencyContainer;
use icn_core::interfaces::identity::{IdentityProvider, IdentityDetails, IdentityRegistration, Result as IdentityResult};
use icn_core::interfaces::storage::{StorageProvider, StorageOptions, Result as StorageResult};
use icn_core::interfaces::reputation::{ReputationProvider, TrustScore, Evidence, ValidationContext, ValidationResponse, Result as ReputationResult};
use icn_core::interfaces::economic::{EconomicProvider, AccountId};
use icn_economic::{MutualCreditConfig, MutualCreditFactory};
use serde::{Serialize, de::DeserializeOwned};
use std::collections::HashMap;

/// A simple in-memory implementation of the IdentityProvider
struct MockIdentityProvider {
    identities: HashMap<String, IdentityDetails>,
}

impl MockIdentityProvider {
    fn new() -> Self {
        Self {
            identities: HashMap::new(),
        }
    }
}

#[async_trait]
impl IdentityProvider for MockIdentityProvider {
    async fn validate_identity(&self, did: &str) -> IdentityResult<bool> {
        Ok(self.identities.contains_key(did))
    }
    
    async fn get_identity_details(&self, did: &str) -> IdentityResult<Option<IdentityDetails>> {
        Ok(self.identities.get(did).cloned())
    }
    
    async fn register_identity(&self, details: IdentityRegistration) -> IdentityResult<String> {
        let did = format!("did:icn:{}", details.node_id);
        Ok(did)
    }
    
    async fn verify_signature(&self, did: &str, message: &[u8], signature: &[u8]) -> IdentityResult<bool> {
        // In a real implementation, we would verify the signature
        Ok(true)
    }
}

/// A simple in-memory implementation of the StorageProvider
struct MockStorageProvider {
    data: HashMap<String, Vec<u8>>,
}

impl MockStorageProvider {
    fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }
}

#[async_trait]
impl StorageProvider for MockStorageProvider {
    async fn store<T: Serialize + Send + Sync>(&self, key: &str, value: &T, _options: Option<StorageOptions>) -> StorageResult<()> {
        let json = serde_json::to_vec(value)?;
        let mut data = self.data.clone();
        data.insert(key.to_string(), json);
        Ok(())
    }
    
    async fn retrieve<T: DeserializeOwned + Send + Sync>(&self, key: &str, _options: Option<StorageOptions>) -> StorageResult<Option<T>> {
        if let Some(data) = self.data.get(key) {
            let value = serde_json::from_slice(data)?;
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }
    
    async fn delete(&self, key: &str, _options: Option<StorageOptions>) -> StorageResult<bool> {
        Ok(self.data.contains_key(key))
    }
    
    async fn exists(&self, key: &str, _options: Option<StorageOptions>) -> StorageResult<bool> {
        Ok(self.data.contains_key(key))
    }
    
    async fn list_keys(&self, pattern: &str, _options: Option<StorageOptions>) -> StorageResult<Vec<String>> {
        let keys: Vec<String> = self.data.keys()
            .filter(|k| k.contains(pattern))
            .cloned()
            .collect();
        Ok(keys)
    }
}

/// A simple implementation of the ReputationProvider
struct MockReputationProvider;

impl MockReputationProvider {
    fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ReputationProvider for MockReputationProvider {
    async fn get_trust_score(&self, _did: &str) -> ReputationResult<Option<TrustScore>> {
        // Always return a high trust score for testing
        Ok(Some(TrustScore {
            score: 0.9,
            confidence: 0.8,
            last_updated: chrono::Utc::now().timestamp() as u64,
        }))
    }
    
    async fn record_interaction(&self, _evidence: Evidence) -> ReputationResult<()> {
        // Do nothing in mock
        Ok(())
    }
    
    async fn validate_entity(&self, _did: &str, _context: ValidationContext) -> ReputationResult<ValidationResponse> {
        // Always validate for testing
        Ok(ValidationResponse {
            is_valid: true,
            trust_score: Some(TrustScore {
                score: 0.9,
                confidence: 0.8,
                last_updated: chrono::Utc::now().timestamp() as u64,
            }),
            reason: None,
        })
    }
    
    async fn get_reputation_metrics(&self, _start_time: u64, _end_time: u64) -> ReputationResult<HashMap<String, f64>> {
        Ok(HashMap::new())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create our dependency container
    let mut container = DependencyContainer::new();
    
    // Register our mock implementations
    let identity_provider = Arc::new(MockIdentityProvider::new());
    let storage_provider = Arc::new(MockStorageProvider::new());
    let reputation_provider = Arc::new(MockReputationProvider::new());
    
    // Register implementations with their interface types
    container.register::<dyn IdentityProvider>(identity_provider);
    container.register::<dyn StorageProvider>(storage_provider);
    container.register::<dyn ReputationProvider>(reputation_provider);
    
    // Create a MutualCreditSystem using our factory
    let config = MutualCreditConfig {
        default_credit_limit:
        1000,
        fee_percentage: 0.1,
        fee_recipient: Some("system".to_string()),
        currency_code: "ICN".to_string(),
        storage_namespace: "test".to_string(),
    };
    
    let economic_provider = MutualCreditFactory::create_from_container(&container, config)?;
    
    // Now we can use our economic provider through its interface
    let account_id = economic_provider.create_account("did:icn:test", None).await
        .map_err(|e| format!("Failed to create account: {:?}", e))?;
    
    println!("Created account: {}", account_id.0);
    
    // Get the balance
    let balance = economic_provider.get_balance(&account_id).await
        .map_err(|e| format!("Failed to get balance: {:?}", e))?;
    
    println!("Account balance: {} {}", balance.available.value, balance.available.currency);
    println!("Credit limit: {}", balance.credit_limit);
    
    // Our components are using interfaces, not direct references to each other!
    println!("Dependency inversion successfully demonstrated!");
    
    Ok(())
} 