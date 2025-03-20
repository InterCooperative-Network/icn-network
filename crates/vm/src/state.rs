//! State management for the VM

use std::sync::Arc;
use std::collections::HashMap;
use dashmap::DashMap;
use serde::{Serialize, Deserialize};
use async_trait::async_trait;

use icn_common::types::{Value, DID};
use crate::error::{Result, VMError};

/// Trait for accessing VM state
#[async_trait]
pub trait StateAccess: Send + Sync {
    /// Get a value from state
    async fn get(&self, collection: &str, key: &str) -> Result<Option<Value>>;
    
    /// Put a value into state
    async fn put(&self, collection: &str, key: &str, value: Value) -> Result<()>;
    
    /// Delete a value from state
    async fn delete(&self, collection: &str, key: &str) -> Result<()>;
    
    /// List keys in a collection
    async fn list(&self, collection: &str) -> Result<Vec<String>>;
}

/// State manager for the VM
pub struct StateManager {
    /// Storage collections
    collections: DashMap<String, DashMap<String, Value>>,
}

impl StateManager {
    /// Create a new state manager
    pub fn new() -> Self {
        Self {
            collections: DashMap::new(),
        }
    }
    
    /// Get or create a collection
    fn get_collection(&self, name: &str) -> Arc<DashMap<String, Value>> {
        if let Some(collection) = self.collections.get(name) {
            Arc::new(collection.clone())
        } else {
            let collection = DashMap::new();
            self.collections.insert(name.to_string(), collection.clone());
            Arc::new(collection)
        }
    }
}

#[async_trait]
impl StateAccess for StateManager {
    async fn get(&self, collection: &str, key: &str) -> Result<Option<Value>> {
        let collection = self.get_collection(collection);
        Ok(collection.get(key).map(|v| v.clone()))
    }
    
    async fn put(&self, collection: &str, key: &str, value: Value) -> Result<()> {
        let collection = self.get_collection(collection);
        collection.insert(key.to_string(), value);
        Ok(())
    }
    
    async fn delete(&self, collection: &str, key: &str) -> Result<()> {
        let collection = self.get_collection(collection);
        collection.remove(key);
        Ok(())
    }
    
    async fn list(&self, collection: &str) -> Result<Vec<String>> {
        let collection = self.get_collection(collection);
        let keys = collection.iter().map(|kv| kv.key().clone()).collect();
        Ok(keys)
    }
}

#[derive(Error, Debug)]
pub enum StateError {
    #[error("State not found: {0}")]
    NotFound(String),
    
    #[error("Invalid state: {0}")]
    Invalid(String),
    
    #[error("Access denied: {0}")]
    AccessDenied(String),
}

/// VM State holds the current state of the virtual machine
#[derive(Debug)]
pub struct VMState {
    /// Active proposals
    pub proposals: DashMap<String, Proposal>,
    /// Registered assets
    pub assets: DashMap<String, Asset>,
    /// Defined roles
    pub roles: DashMap<String, Role>,
    /// Membership configurations
    pub memberships: DashMap<String, Membership>,
    /// Federations
    pub federations: DashMap<String, Federation>,
    /// Credit systems
    pub credit_systems: DashMap<String, CreditSystem>,
    /// Individual member records
    pub members: DashMap<String, Member>,
    /// Votes on proposals
    pub votes: DashMap<String, HashMap<String, Vote>>,
    /// General key-value store for other state
    pub store: DashMap<String, Value>,
}

/// Changes made to the VM state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StateChange {
    /// Add/Update a proposal
    UpdateProposal(Proposal),
    /// Add/Update an asset
    UpdateAsset(Asset),
    /// Add/Update a role
    UpdateRole(Role),
    /// Add/Update a membership
    UpdateMembership(Membership),
    /// Add/Update a federation
    UpdateFederation(Federation),
    /// Add/Update a credit system
    UpdateCreditSystem(CreditSystem),
    /// Add/Update a member
    UpdateMember(Member),
    /// Add/Update a vote
    UpdateVote(Vote),
    /// Set a value in the store
    SetValue(String, Value),
    /// Delete a value from the store
    DeleteValue(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Member {
    pub id: String,
    pub did: String,
    pub name: String,
    pub roles: Vec<String>,
    pub joined_date: String, // ISO format
    pub credentials: HashMap<String, String>,
    pub attributes: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vote {
    pub member_id: String,
    pub proposal_id: String,
    pub vote: VoteValue,
    pub timestamp: String, // ISO format
    pub weight: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VoteValue {
    Yes,
    No,
    Abstain,
    RankedChoice(Vec<String>),
    WeightedChoice(HashMap<String, f64>),
}

// Import from dsl crate or define here
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    pub title: String,
    pub description: String,
    pub quorum: f64,
    pub threshold: Option<f64>,
    pub voting_method: VotingMethod,
    pub required_role: Option<String>,
    pub voting_period: Option<String>,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
    pub execution: Vec<ExecutionStep>,
    pub rejection: Option<Vec<ExecutionStep>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VotingMethod {
    Majority,
    Consensus,
    RankedChoice,
    Quadratic,
    SingleChoice,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStep {
    pub function: String,
    pub args: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub name: String,
    pub symbol: Option<String>,
    pub description: Option<String>,
    pub total_supply: Option<f64>,
    pub divisible: Option<bool>,
    pub transferable: Option<bool>,
    pub metadata: Option<HashMap<String, Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub name: String,
    pub description: Option<String>,
    pub permissions: Vec<String>,
    pub parent_role: Option<String>,
    pub max_members: Option<u32>,
    pub assignable_by: Option<Vec<String>>,
    pub attributes: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Membership {
    pub name: String,
    pub onboarding: OnboardingMethod,
    pub default_role: Option<String>,
    pub max_members: Option<u32>,
    pub voting_rights: Option<bool>,
    pub credentials: Option<Vec<String>>,
    pub attributes: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OnboardingMethod {
    Open,
    Invitation,
    ApprovalVote,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Federation {
    pub name: String,
    pub description: Option<String>,
    pub members: Option<Vec<String>>,
    pub governance_model: Option<String>,
    pub attributes: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreditSystem {
    pub name: String,
    pub currency_name: Option<String>,
    pub currency_symbol: Option<String>,
    pub initial_supply: Option<f64>,
    pub issuance_policy: Option<String>,
    pub attributes: HashMap<String, Value>,
}

impl VMState {
    pub fn new() -> Self {
        Self {
            proposals: DashMap::new(),
            assets: DashMap::new(),
            roles: DashMap::new(),
            memberships: DashMap::new(),
            federations: DashMap::new(),
            credit_systems: DashMap::new(),
            members: DashMap::new(),
            votes: DashMap::new(),
            store: DashMap::new(),
        }
    }
} 