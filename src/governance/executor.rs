use super::primitives::*;
use std::error::Error;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use async_trait::async_trait;

pub struct BasicActionExecutor {
    identities: Arc<RwLock<HashMap<String, Identity>>>,
    ledger: Arc<RwLock<HashMap<String, u64>>>,  // Simple asset ledger
    policies: Arc<RwLock<HashMap<String, String>>>,
}

impl BasicActionExecutor {
    pub fn new() -> Self {
        Self {
            identities: Arc::new(RwLock::new(HashMap::new())),
            ledger: Arc::new(RwLock::new(HashMap::new())),
            policies: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl ActionExecutor for BasicActionExecutor {
    async fn execute(&self, action: &Action) -> Result<(), Box<dyn Error>> {
        match action {
            Action::AllocateFunds { asset, amount, recipient } => {
                let mut ledger = self.ledger.write().await;
                let balance = ledger.entry(recipient.clone())
                    .or_insert(0);
                *balance += amount;
                Ok(())
            }

            Action::UpdateRole { identity, role, add } => {
                let mut identities = self.identities.write().await;
                if let Some(identity) = identities.get_mut(identity) {
                    if *add {
                        identity.roles.insert(role.clone());
                    } else {
                        identity.roles.remove(role);
                    }
                    Ok(())
                } else {
                    Err("Identity not found".into())
                }
            }

            Action::UpdatePolicy { key, value } => {
                let mut policies = self.policies.write().await;
                policies.insert(key.clone(), value.clone());
                Ok(())
            }

            Action::Custom { action_type, params } => {
                // Log custom action for audit
                println!(
                    "Executing custom action: {} with params: {:?}",
                    action_type, params
                );
                Ok(())
            }
        }
    }
} 