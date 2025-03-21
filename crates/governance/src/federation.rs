use std::error::Error;
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use icn_core::storage::Storage;
use icn_identity::Identity;
use crate::{GovernanceError, Proposal, ProposalType, ProposalStatus};
use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::{info, debug, error};

// Federation governance error types
#[derive(Debug, Error)]
pub enum FederationGovernanceError {
    #[error("Invalid proposal: {0}")]
    InvalidProposal(String),
    #[error("Invalid vote: {0}")]
    InvalidVote(String),
    #[error("Proposal not found: {0}")]
    ProposalNotFound(String),
    #[error("Voting period expired: {0}")]
    VotingPeriodExpired(String),
    #[error("Insufficient votes: {0}")]
    InsufficientVotes(String),
    #[error("Invalid quorum: {0}")]
    InvalidQuorum(String),
    #[error("Dispute not found: {0}")]
    DisputeNotFound(String),
    #[error("Invalid resolution: {0}")]
    InvalidResolution(String),
    #[error("Invalid coordination: {0}")]
    InvalidCoordination(String),
    #[error("Coordination not found: {0}")]
    CoordinationNotFound(String),
    #[error("Invalid federation: {0}")]
    InvalidFederation(String),
    #[error("Insufficient federations: {0}")]
    InsufficientFederations(String),
    #[error("Coordination expired: {0}")]
    CoordinationExpired(String),
    #[error("Invalid consensus: {0}")]
    InvalidConsensus(String),
}

// Cross-federation coordination types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CoordinationType {
    PolicyAlignment,
    ResourceSharing,
    DisputeResolution,
    EmergencyResponse,
    SystemUpgrade,
}

// Cross-federation coordination status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CoordinationStatus {
    Draft,
    Active,
    ConsensusReached,
    Implemented,
    Failed,
    Expired,
}

// Cross-federation coordination structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossFederationCoordination {
    pub id: String,
    pub coordination_type: CoordinationType,
    pub title: String,
    pub description: String,
    pub created_by: String,
    pub created_at: u64,
    pub expires_at: u64,
    pub required_federations: u64,
    pub participating_federations: Vec<String>,
    pub status: CoordinationStatus,
    pub proposals: Vec<Proposal>,
    pub consensus: Option<Consensus>,
}

// Consensus structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Consensus {
    pub reached_at: u64,
    pub agreed_proposals: Vec<String>,
    pub implementation_plan: Vec<String>,
    pub signatures: Vec<ConsensusSignature>,
}

// Consensus signature structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusSignature {
    pub federation_id: String,
    pub signature: Vec<u8>,
    pub timestamp: u64,
}

// Evidence for governance disputes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceEvidence {
    pub id: String,
    pub proposal_id: String,
    pub evidence_type: String,
    pub data: serde_json::Value,
    pub timestamp: u64,
}

// Dispute resolution for governance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dispute {
    pub id: String,
    pub proposal_id: String,
    pub raised_by: String,
    pub reason: String,
    pub evidence: Vec<GovernanceEvidence>,
    pub resolution: Option<DisputeResolution>,
    pub created_at: u64,
    pub status: DisputeStatus,
}

// Resolution for a dispute
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisputeResolution {
    pub resolved_by: String,
    pub decision: String,
    pub evidence: Vec<GovernanceEvidence>,
    pub timestamp: u64,
    pub signature: Vec<u8>,
}

// Dispute status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DisputeStatus {
    Open,
    UnderReview,
    Resolved,
    Dismissed,
    Escalated,
}

// Federation governance system
pub struct FederationGovernance {
    identity: Arc<Identity>,
    storage: Arc<dyn Storage>,
}

impl FederationGovernance {
    // Create a new governance system
    pub fn new(identity: Arc<Identity>, storage: Arc<dyn Storage>) -> Self {
        Self {
            identity,
            storage,
        }
    }

    /// Set the reputation system to use for governance scoring
    pub fn set_reputation_system(&mut self, _reputation: Arc<dyn std::any::Any>) {
        // Implementation to be added
    }

    // Create a new proposal
    pub fn create_proposal(
        &self,
        federation_id: &str,
        proposal_type: ProposalType,
        title: &str,
        description: &str,
        voting_duration: u64,
        quorum: u64,
        changes: serde_json::Value,
    ) -> Result<Proposal, Box<dyn Error>> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();

        let proposal = Proposal {
            id: format!("prop-{}", now),
            federation_id: federation_id.to_string(),
            proposal_type,
            title: title.to_string(),
            description: description.to_string(),
            creator_did: self.identity.did.clone(),
            created_at: now,
            voting_end: now + voting_duration,
            quorum,
            votes_yes: 0,
            votes_no: 0,
            status: ProposalStatus::Voting,
            changes,
        };

        // Store the proposal
        self.storage.put_json(&format!("proposals/{}", proposal.id), &proposal)?;

        // Add to list of proposals for this federation
        let federation_proposals_key = format!("federation_proposals/{}", federation_id);
        let mut proposal_ids: Vec<String> = self.storage
            .get_json(&federation_proposals_key)
            .unwrap_or_else(|_| Vec::new());
        
        proposal_ids.push(proposal.id.clone());
        self.storage.put_json(&federation_proposals_key, &proposal_ids)?;

        Ok(proposal)
    }

    // Create a new cross-federation coordination
    pub fn create_coordination(
        &self,
        coordination_type: CoordinationType,
        title: &str,
        description: &str,
        duration: u64,
        required_federations: u64,
    ) -> Result<CrossFederationCoordination, Box<dyn Error>> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();

        let coordination = CrossFederationCoordination {
            id: format!("coord-{}", now),
            coordination_type,
            title: title.to_string(),
            description: description.to_string(),
            created_by: self.identity.did.clone(),
            created_at: now,
            expires_at: now + duration,
            required_federations,
            participating_federations: vec![self.identity.coop_id.clone()],
            status: CoordinationStatus::Draft,
            proposals: Vec::new(),
            consensus: None,
        };

        // Store the coordination
        self.storage.put_json(
            &format!("cross_federation_coordinations/{}", coordination.id),
            &coordination,
        )?;

        Ok(coordination)
    }

    // Join a cross-federation coordination
    pub fn join_coordination(
        &self,
        coordination_id: &str,
    ) -> Result<(), Box<dyn Error>> {
        let mut coordination: CrossFederationCoordination = self.storage.get_json(
            &format!("cross_federation_coordinations/{}", coordination_id),
        )?;

        // Check if coordination is still active
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();
        if now > coordination.expires_at {
            return Err(Box::new(FederationGovernanceError::CoordinationExpired(
                "Coordination period has ended".to_string(),
            )));
        }

        // Check if federation is already participating
        if coordination.participating_federations.contains(&self.identity.coop_id) {
            return Err(Box::new(FederationGovernanceError::InvalidFederation(
                "Federation is already participating".to_string(),
            )));
        }

        // Add federation to participants
        coordination.participating_federations.push(self.identity.coop_id.clone());
        coordination.status = CoordinationStatus::Active;

        // Store updated coordination
        self.storage.put_json(
            &format!("cross_federation_coordinations/{}", coordination_id),
            &coordination,
        )?;

        Ok(())
    }

    // Submit a proposal to a coordination
    pub fn submit_proposal(
        &self,
        coordination_id: &str,
        proposal: Proposal,
    ) -> Result<(), Box<dyn Error>> {
        let mut coordination: CrossFederationCoordination = self.storage.get_json(
            &format!("cross_federation_coordinations/{}", coordination_id),
        )?;

        // Verify federation is participating
        if !coordination.participating_federations.contains(&self.identity.coop_id) {
            return Err(Box::new(FederationGovernanceError::InvalidFederation(
                "Federation is not participating".to_string(),
            )));
        }

        // Add proposal to coordination
        coordination.proposals.push(proposal);

        // Store updated coordination
        self.storage.put_json(
            &format!("cross_federation_coordinations/{}", coordination_id),
            &coordination,
        )?;

        Ok(())
    }

    // Reach consensus on coordination
    pub fn reach_consensus(
        &self,
        coordination_id: &str,
        agreed_proposals: Vec<String>,
        implementation_plan: Vec<String>,
    ) -> Result<(), Box<dyn Error>> {
        let mut coordination: CrossFederationCoordination = self.storage.get_json(
            &format!("cross_federation_coordinations/{}", coordination_id),
        )?;

        // Verify federation is participating
        if !coordination.participating_federations.contains(&self.identity.coop_id) {
            return Err(Box::new(FederationGovernanceError::InvalidFederation(
                "Federation is not participating".to_string(),
            )));
        }

        // Check if enough federations are participating
        if coordination.participating_federations.len() < coordination.required_federations as usize {
            return Err(Box::new(FederationGovernanceError::InsufficientFederations(
                "Not enough federations participating".to_string(),
            )));
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();

        // Create and sign consensus
        let consensus_data = serde_json::to_vec(&(coordination_id, &agreed_proposals, &implementation_plan, now))?;
        let signature = self.identity.sign(&consensus_data)?;

        let consensus_signature = ConsensusSignature {
            federation_id: self.identity.coop_id.clone(),
            signature: signature.to_bytes().to_vec(),
            timestamp: now,
        };

        // Create consensus if it doesn't exist
        let mut consensus = coordination.consensus.unwrap_or(Consensus {
            reached_at: now,
            agreed_proposals,
            implementation_plan,
            signatures: Vec::new(),
        });

        // Add signature to consensus
        consensus.signatures.push(consensus_signature);

        // Update coordination with consensus
        coordination.consensus = Some(consensus);
        coordination.status = CoordinationStatus::ConsensusReached;

        // Store updated coordination
        self.storage.put_json(
            &format!("cross_federation_coordinations/{}", coordination_id),
            &coordination,
        )?;

        Ok(())
    }

    // Implement consensus
    pub fn implement_consensus(
        &self,
        coordination_id: &str,
    ) -> Result<(), Box<dyn Error>> {
        let coordination: CrossFederationCoordination = self.storage.get_json(
            &format!("cross_federation_coordinations/{}", coordination_id),
        )?;

        // Verify federation is participating
        if !coordination.participating_federations.contains(&self.identity.coop_id) {
            return Err(Box::new(FederationGovernanceError::InvalidFederation(
                "Federation is not participating".to_string(),
            )));
        }

        // Verify consensus exists
        let consensus = coordination.consensus.ok_or_else(|| {
            FederationGovernanceError::InvalidConsensus("No consensus reached".to_string())
        })?;

        // Verify all required federations have signed
        if consensus.signatures.len() < coordination.required_federations as usize {
            return Err(Box::new(FederationGovernanceError::InsufficientFederations(
                "Not all required federations have signed".to_string(),
            )));
        }

        // Implement agreed proposals
        for proposal_id in &consensus.agreed_proposals {
            let proposal = self.get_proposal(proposal_id)?;
            self.apply_proposal_changes(&proposal)?;
        }

        // Update coordination status
        let mut updated_coordination = coordination;
        updated_coordination.status = CoordinationStatus::Implemented;
        self.storage.put_json(
            &format!("cross_federation_coordinations/{}", coordination_id),
            &updated_coordination,
        )?;

        Ok(())
    }

    // Helper function to apply proposal changes
    fn apply_proposal_changes(&self, proposal: &Proposal) -> Result<(), Box<dyn Error>> {
        // Apply changes based on proposal type
        match proposal.proposal_type {
            ProposalType::PolicyChange => {
                // Apply policy changes
                let policy_key = format!("federation_policy/{}", proposal.federation_id);
                self.storage.put_json(&policy_key, &proposal.changes)?;
            }
            ProposalType::MemberAddition => {
                // Add new member
                let members_key = format!("federation_members/{}", proposal.federation_id);
                let mut members: Vec<String> = self.storage
                    .get_json(&members_key)
                    .unwrap_or_else(|_| Vec::new());
                
                if let Some(new_member) = proposal.changes.get("member_id") {
                    members.push(new_member.as_str().unwrap().to_string());
                    self.storage.put_json(&members_key, &members)?;
                }
            }
            ProposalType::MemberRemoval => {
                // Remove member
                let members_key = format!("federation_members/{}", proposal.federation_id);
                let mut members: Vec<String> = self.storage
                    .get_json(&members_key)
                    .unwrap_or_else(|_| Vec::new());
                
                if let Some(member_to_remove) = proposal.changes.get("member_id") {
                    members.retain(|m| m != member_to_remove.as_str().unwrap());
                    self.storage.put_json(&members_key, &members)?;
                }
            }
            _ => {
                // Handle other proposal types
                let changes_key = format!("federation_changes/{}", proposal.id);
                self.storage.put_json(&changes_key, &proposal.changes)?;
            }
        }

        Ok(())
    }

    // Get a proposal by ID
    pub fn get_proposal(&self, proposal_id: &str) -> Result<Proposal, Box<dyn Error>> {
        self.storage.get_json(&format!("proposals/{}", proposal_id))
    }

    // Get a coordination by ID
    pub fn get_coordination(&self, coordination_id: &str) -> Result<CrossFederationCoordination, Box<dyn Error>> {
        self.storage.get_json(&format!("cross_federation_coordinations/{}", coordination_id))
    }

    /// Vote on a proposal
    pub async fn vote(&self, proposal_id: &str, vote: bool) -> Result<(), Box<dyn Error>> {
        // Implementation to be added
        Ok(())
    }
    
    /// Add a deliberation to a proposal
    pub async fn add_deliberation(
        &self,
        proposal_id: &str,
        comment: &str,
        references: Vec<String>,
    ) -> Result<Deliberation, Box<dyn Error>> {
        // Implementation to be added
        let deliberation = Deliberation {
            id: format!("delib-{}", proposal_id),
            proposal_id: proposal_id.to_string(),
            member_did: self.identity.did.clone(),
            comment: comment.to_string(),
            references,
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            votes: 0,
        };
        
        Ok(deliberation)
    }
    
    /// Get all deliberations for a proposal
    pub fn get_deliberations(&self, proposal_id: &str) -> Result<Vec<Deliberation>, Box<dyn Error>> {
        // Implementation to be added
        Ok(vec![])
    }
    
    /// Calculate governance participation score for a member
    pub async fn calculate_governance_score(
        &self,
        member_did: &str,
    ) -> Result<GovernanceParticipationScore, Box<dyn Error>> {
        // Implementation to be added
        let score = GovernanceParticipationScore {
            member_did: member_did.to_string(),
            proposals_created: 0,
            proposals_voted: 0,
            deliberations_count: 0,
            last_participation: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };
        
        Ok(score)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use icn_core::storage::MemoryStorage;

    #[test]
    fn test_proposal_creation() {
        let storage = Arc::new(MemoryStorage::new());
        let identity = Arc::new(Identity::new("test-coop".to_string()));
        let governance = FederationGovernance::new(identity, storage);

        let proposal = governance.create_proposal(
            "test-federation",
            ProposalType::PolicyChange,
            "Test Proposal",
            "Test Description",
            3600,
            2,
            serde_json::json!({
                "policy": "test-policy"
            }),
        ).unwrap();

        assert_eq!(proposal.title, "Test Proposal");
        assert_eq!(proposal.description, "Test Description");
        assert_eq!(proposal.federation_id, "test-federation");
    }

    #[test]
    fn test_coordination_creation() {
        let storage = Arc::new(MemoryStorage::new());
        let identity = Arc::new(Identity::new("test-coop".to_string()));
        let governance = FederationGovernance::new(identity, storage);

        let coordination = governance.create_coordination(
            CoordinationType::PolicyAlignment,
            "Test Coordination",
            "Test Description",
            3600,
            2,
        ).unwrap();

        assert_eq!(coordination.title, "Test Coordination");
        assert_eq!(coordination.description, "Test Description");
        assert_eq!(coordination.required_federations, 2);
    }
}

// Re-export federation types
pub mod coordination {
    pub use super::*;
    
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
}

pub use coordination::*;

// Import storage manager types
pub mod storage_manager {
    use serde::{Serialize, Deserialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct FederationStorageConfig {
        pub storage_path: String,
        pub max_size_gb: f64,
        pub replication_factor: u32,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct FederationStorageStats {
        pub total_bytes_used: u64,
        pub total_files: u32,
        pub replication_health: f64,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct StorageRoute {
        pub resource_id: String,
        pub path: String,
        pub access_method: String,
    }
}

/// Federation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationConfig {
    pub name: String,
    pub description: String,
    pub federation_id: String,
    pub peers: Vec<String>,
    pub storage_path: String,
}

/// Federation state
#[derive(Debug)]
pub struct Federation {
    config: FederationConfig,
    state: Arc<RwLock<FederationState>>,
}

#[derive(Debug, Default)]
struct FederationState {
    is_active: bool,
    connected_peers: Vec<String>,
    resource_usage: std::collections::HashMap<String, f64>,
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
    
    pub async fn get_federation_info(&self, federation_id: &str) -> Result<FederationInfo, Box<dyn Error>> {
        let federations = self.federations.read().await;
        federations.get(federation_id)
            .cloned()
            .ok_or_else(|| "Federation not found".into())
    }
    
    pub async fn list_federations(&self) -> Result<Vec<FederationInfo>, Box<dyn Error>> {
        let federations = self.federations.read().await;
        Ok(federations.values().cloned().collect())
    }
    
    pub async fn update_federation(
        &self,
        federation_id: &str,
        description: Option<String>,
        members: Option<Vec<String>>,
        policies: Option<Vec<FederationPolicy>>,
        metadata: Option<serde_json::Value>,
    ) -> Result<(), Box<dyn Error>> {
        let mut federations = self.federations.write().await;
        
        let federation = federations.get_mut(federation_id)
            .ok_or("Federation not found")?;
            
        if let Some(desc) = description {
            federation.description = desc;
        }
        
        if let Some(m) = members {
            federation.members = m;
        }
        
        if let Some(p) = policies {
            federation.policies = p;
        }
        
        if let Some(md) = metadata {
            federation.metadata = md;
        }
        
        federation.last_active = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();
            
        Ok(())
    }
    
    pub async fn terminate_agreement(
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
        
        agreement.status = AgreementStatus::Terminated;
        agreement.updated_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();
            
        Ok(())
    }
}

/// Federation system for managing cross-cooperative coordination
///
/// This system manages federations of cooperatives, including membership,
/// resource sharing, and coordination activities.
pub struct FederationSystem {
    identity: Arc<Identity>,
    storage: Arc<dyn Storage>,
    economic: Arc<dyn std::any::Any>, // Generic economic system reference
}

impl FederationSystem {
    /// Create a new FederationSystem instance
    pub fn new(
        identity: Arc<Identity>,
        storage: Arc<dyn Storage>,
        economic: Arc<dyn std::any::Any>,
    ) -> Self {
        Self {
            identity,
            storage,
            economic,
        }
    }
    
    /// Start the federation system
    pub async fn start(&self) -> Result<(), Box<dyn Error>> {
        // Implementation to be added
        Ok(())
    }
    
    /// Stop the federation system
    pub async fn stop(&self) -> Result<(), Box<dyn Error>> {
        // Implementation to be added
        Ok(())
    }
}

/// Deliberation on a proposal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deliberation {
    pub id: String,
    pub proposal_id: String,
    pub member_did: String,
    pub comment: String,
    pub references: Vec<String>,
    pub created_at: u64,
    pub votes: usize,
}

/// Score for governance participation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceParticipationScore {
    pub member_did: String,
    pub proposals_created: usize,
    pub proposals_voted: usize,
    pub deliberations_count: usize,
    pub last_participation: u64,
} 