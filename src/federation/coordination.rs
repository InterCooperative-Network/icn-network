use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use tokio::sync::RwLock;
use std::sync::Arc;
use std::error::Error;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub members: Vec<String>, // DIDs of member cooperatives
    pub resources: Vec<String>, // Resource IDs shared with federation
    pub policies: Vec<FederationPolicy>,
    pub trust_score: f64,
    pub last_active: u64,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationPolicy {
    pub id: String,
    pub policy_type: PolicyType,
    pub parameters: serde_json::Value,
    pub status: PolicyStatus,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PolicyType {
    ResourceSharing {
        max_share_percentage: f64,
        priority_levels: Vec<String>,
    },
    GovernanceParticipation {
        voting_weight: f64,
        proposal_rights: Vec<String>,
    },
    TrustManagement {
        min_trust_score: f64,
        reputation_factors: Vec<String>,
    },
    DisputeResolution {
        resolution_methods: Vec<String>,
        arbitrators: Vec<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PolicyStatus {
    Active,
    Pending,
    Suspended,
    Archived,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationAgreement {
    pub id: String,
    pub federation_a: String,
    pub federation_b: String,
    pub shared_resources: Vec<SharedResource>,
    pub shared_policies: Vec<FederationPolicy>,
    pub status: AgreementStatus,
    pub created_at: u64,
    pub updated_at: u64,
    pub valid_until: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedResource {
    pub resource_id: String,
    pub share_percentage: f64,
    pub priority_access: bool,
    pub usage_limits: ResourceUsageLimits,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsageLimits {
    pub max_concurrent_allocations: u32,
    pub max_duration_per_allocation: u64,
    pub max_total_duration_per_day: u64,
    pub restricted_hours: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AgreementStatus {
    Proposed,
    Active,
    Suspended,
    Terminated,
}

pub struct FederationCoordinator {
    federations: Arc<RwLock<HashMap<String, FederationInfo>>>,
    agreements: Arc<RwLock<HashMap<String, FederationAgreement>>>,
}

impl FederationCoordinator {
    pub fn new() -> Self {
        FederationCoordinator {
            federations: Arc::new(RwLock::new(HashMap::new())),
            agreements: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register_federation(
        &self,
        name: &str,
        description: &str,
        members: Vec<String>,
        policies: Vec<FederationPolicy>,
        metadata: serde_json::Value,
    ) -> Result<String, Box<dyn Error>> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();

        let federation = FederationInfo {
            id: format!("fed-{}", now),
            name: name.to_string(),
            description: description.to_string(),
            members,
            resources: Vec::new(),
            policies,
            trust_score: 1.0,
            last_active: now,
            metadata,
        };

        let mut federations = self.federations.write().await;
        federations.insert(federation.id.clone(), federation.clone());

        Ok(federation.id)
    }

    pub async fn propose_agreement(
        &self,
        federation_a: &str,
        federation_b: &str,
        shared_resources: Vec<SharedResource>,
        shared_policies: Vec<FederationPolicy>,
        valid_duration: u64,
    ) -> Result<String, Box<dyn Error>> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();

        // Verify both federations exist
        let federations = self.federations.read().await;
        if !federations.contains_key(federation_a) || !federations.contains_key(federation_b) {
            return Err("One or both federations not found".into());
        }

        let agreement = FederationAgreement {
            id: format!("agreement-{}", now),
            federation_a: federation_a.to_string(),
            federation_b: federation_b.to_string(),
            shared_resources,
            shared_policies,
            status: AgreementStatus::Proposed,
            created_at: now,
            updated_at: now,
            valid_until: now + valid_duration,
        };

        let mut agreements = self.agreements.write().await;
        agreements.insert(agreement.id.clone(), agreement.clone());

        Ok(agreement.id)
    }

    pub async fn activate_agreement(
        &self,
        agreement_id: &str,
        federation_id: &str,
    ) -> Result<(), Box<dyn Error>> {
        let mut agreements = self.agreements.write().await;
        let agreement = agreements.get_mut(agreement_id)
            .ok_or("Agreement not found")?;

        // Verify the federation is part of the agreement
        if agreement.federation_a != federation_id && agreement.federation_b != federation_id {
            return Err("Federation not part of agreement".into());
        }

        // If both federations have approved, activate the agreement
        if agreement.status == AgreementStatus::Proposed {
            agreement.status = AgreementStatus::Active;
            agreement.updated_at = SystemTime::now()
                .duration_since(UNIX_EPOCH)?
                .as_secs();
        }

        Ok(())
    }

    pub async fn update_trust_score(
        &self,
        federation_id: &str,
        interaction_score: f64,
    ) -> Result<(), Box<dyn Error>> {
        let mut federations = self.federations.write().await;
        let federation = federations.get_mut(federation_id)
            .ok_or("Federation not found")?;

        // Update trust score with exponential moving average
        const ALPHA: f64 = 0.3; // Weight for new score
        federation.trust_score = (1.0 - ALPHA) * federation.trust_score + ALPHA * interaction_score;
        federation.last_active = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();

        Ok(())
    }

    pub async fn get_shared_resources(
        &self,
        federation_id: &str,
    ) -> Result<Vec<SharedResource>, Box<dyn Error>> {
        let agreements = self.agreements.read().await;
        let mut shared_resources = Vec::new();

        for agreement in agreements.values() {
            if (agreement.federation_a == federation_id || agreement.federation_b == federation_id)
                && agreement.status == AgreementStatus::Active {
                shared_resources.extend(agreement.shared_resources.clone());
            }
        }

        Ok(shared_resources)
    }

    pub async fn verify_resource_access(
        &self,
        federation_id: &str,
        resource_id: &str,
    ) -> Result<bool, Box<dyn Error>> {
        let agreements = self.agreements.read().await;
        
        for agreement in agreements.values() {
            if agreement.status == AgreementStatus::Active &&
               (agreement.federation_a == federation_id || agreement.federation_b == federation_id) {
                if agreement.shared_resources.iter().any(|r| r.resource_id == resource_id) {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    pub async fn get_federation_policies(
        &self,
        federation_id: &str,
    ) -> Result<Vec<FederationPolicy>, Box<dyn Error>> {
        let federations = self.federations.read().await;
        let federation = federations.get(federation_id)
            .ok_or("Federation not found")?;

        Ok(federation.policies.clone())
    }

    pub async fn suspend_agreement(
        &self,
        agreement_id: &str,
        federation_id: &str,
        reason: &str,
    ) -> Result<(), Box<dyn Error>> {
        let mut agreements = self.agreements.write().await;
        let agreement = agreements.get_mut(agreement_id)
            .ok_or("Agreement not found")?;

        // Verify the federation is part of the agreement
        if agreement.federation_a != federation_id && agreement.federation_b != federation_id {
            return Err("Federation not part of agreement".into());
        }

        agreement.status = AgreementStatus::Suspended;
        agreement.updated_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();

        // Update metadata with suspension reason
        agreement.shared_policies.push(FederationPolicy {
            id: format!("suspension-{}", agreement.updated_at),
            policy_type: PolicyType::DisputeResolution {
                resolution_methods: vec!["mediation".to_string()],
                arbitrators: Vec::new(),
            },
            parameters: serde_json::json!({
                "reason": reason,
                "suspended_by": federation_id,
            }),
            status: PolicyStatus::Active,
            created_at: agreement.updated_at,
            updated_at: agreement.updated_at,
        });

        Ok(())
    }
} 