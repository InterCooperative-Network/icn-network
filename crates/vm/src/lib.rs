use anyhow::Result;
use dashmap::DashMap;
use icn_dsl::{ASTNode, Value, Proposal, Asset, Role, Membership, Federation, CreditSystem, OnboardingMethod, VotingMethod, ExecutionStep};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use std::collections::HashMap;

#[derive(Debug, Error)]
pub enum VMError {
    #[error("Execution error: {0}")]
    ExecutionError(String),
    #[error("State error: {0}")]
    StateError(String),
    #[error("Permission error: {0}")]
    PermissionError(String),
    #[error("Validation error: {0}")]
    ValidationError(String),
}

/// VM State holds the current state of the virtual machine
#[derive(Debug)]
pub struct VMState {
    /// Active proposals
    proposals: DashMap<String, Proposal>,
    /// Registered assets
    assets: DashMap<String, Asset>,
    /// Defined roles
    roles: DashMap<String, Role>,
    /// Membership configurations
    memberships: DashMap<String, Membership>,
    /// Federations
    federations: DashMap<String, Federation>,
    /// Credit systems
    credit_systems: DashMap<String, CreditSystem>,
    /// Individual member records
    members: DashMap<String, Member>,
    /// Votes on proposals
    votes: DashMap<String, HashMap<String, Vote>>,
    /// General key-value store for other state
    store: DashMap<String, Value>,
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

/// The Virtual Machine for executing governance and economic instructions
pub struct VM {
    /// Current VM state
    state: Arc<VMState>,
    /// Built-in function registry
    functions: DashMap<String, Box<dyn Fn(Vec<Value>) -> Result<Value, VMError> + Send + Sync>>,
}

impl VM {
    pub fn new() -> Self {
        let vm = Self {
            state: Arc::new(VMState::new()),
            functions: DashMap::new(),
        };
        
        vm.register_builtin_functions();
        vm
    }
    
    fn register_builtin_functions(&self) {
        // Register allocate_funds function
        self.functions.insert(
            "allocateFunds".to_string(),
            Box::new(|args| {
                if args.len() != 2 {
                    return Err(VMError::ExecutionError(
                        "allocateFunds requires 2 arguments: budget_name and amount".to_string(),
                    ));
                }
                
                // In a real implementation, this would update some budget state
                Ok(Value::Boolean(true))
            }),
        );
        
        // Register notify_members function
        self.functions.insert(
            "notifyMembers".to_string(),
            Box::new(|args| {
                if args.len() != 1 {
                    return Err(VMError::ExecutionError(
                        "notifyMembers requires 1 argument: message".to_string(),
                    ));
                }
                
                // In a real implementation, this would send notifications
                Ok(Value::Boolean(true))
            }),
        );
        
        // Register add_member function
        self.functions.insert(
            "addMember".to_string(),
            Box::new(|args| {
                if args.len() < 2 {
                    return Err(VMError::ExecutionError(
                        "addMember requires at least 2 arguments: member_id and role".to_string(),
                    ));
                }
                
                // In a real implementation, this would add a member to the system
                Ok(Value::Boolean(true))
            }),
        );
        
        // Register remove_member function
        self.functions.insert(
            "removeMember".to_string(),
            Box::new(|args| {
                if args.len() != 1 {
                    return Err(VMError::ExecutionError(
                        "removeMember requires 1 argument: member_id".to_string(),
                    ));
                }
                
                // In a real implementation, this would remove a member
                Ok(Value::Boolean(true))
            }),
        );
        
        // Register assign_role function
        self.functions.insert(
            "assignRole".to_string(),
            Box::new(|args| {
                if args.len() != 2 {
                    return Err(VMError::ExecutionError(
                        "assignRole requires 2 arguments: member_id and role".to_string(),
                    ));
                }
                
                // In a real implementation, this would assign a role to a member
                Ok(Value::Boolean(true))
            }),
        );
        
        // Register create_asset function
        self.functions.insert(
            "createAsset".to_string(),
            Box::new(|args| {
                if args.len() < 2 {
                    return Err(VMError::ExecutionError(
                        "createAsset requires at least 2 arguments: asset_name and initial_supply".to_string(),
                    ));
                }
                
                // In a real implementation, this would create a new asset
                Ok(Value::Boolean(true))
            }),
        );
        
        // Register transfer_asset function
        self.functions.insert(
            "transferAsset".to_string(),
            Box::new(|args| {
                if args.len() != 3 {
                    return Err(VMError::ExecutionError(
                        "transferAsset requires 3 arguments: from, to, and amount".to_string(),
                    ));
                }
                
                // In a real implementation, this would transfer an asset between accounts
                Ok(Value::Boolean(true))
            }),
        );
        
        // Register create_federation function
        self.functions.insert(
            "createFederation".to_string(),
            Box::new(|args| {
                if args.len() < 2 {
                    return Err(VMError::ExecutionError(
                        "createFederation requires at least 2 arguments: federation_name and governance_model".to_string(),
                    ));
                }
                
                // In a real implementation, this would create a new federation
                Ok(Value::Boolean(true))
            }),
        );
        
        // Register join_federation function
        self.functions.insert(
            "joinFederation".to_string(),
            Box::new(|args| {
                if args.len() != 2 {
                    return Err(VMError::ExecutionError(
                        "joinFederation requires 2 arguments: federation_id and member_id".to_string(),
                    ));
                }
                
                // In a real implementation, this would add a member to a federation
                Ok(Value::Boolean(true))
            }),
        );
    }
    
    /// Execute a parsed AST node
    pub async fn execute(&self, node: ASTNode) -> Result<Value, VMError> {
        match node {
            ASTNode::Proposal(proposal) => self.execute_proposal(proposal).await,
            ASTNode::Asset(asset) => self.execute_asset_definition(asset).await,
            ASTNode::Role(role) => self.execute_role_definition(role).await,
            ASTNode::Membership(membership) => self.execute_membership_definition(membership).await,
            ASTNode::Federation(federation) => self.execute_federation_definition(federation).await,
            ASTNode::CreditSystem(credit_system) => self.execute_credit_system_definition(credit_system).await,
        }
    }
    
    /// Execute a series of execution steps
    async fn execute_steps(&self, steps: &[ExecutionStep]) -> Result<Vec<Value>, VMError> {
        let mut results = Vec::new();
        for step in steps {
            if let Some(func) = self.functions.get(&step.function) {
                results.push(func(step.args.clone())?);
            } else {
                return Err(VMError::ExecutionError(format!(
                    "Unknown function: {}",
                    step.function
                )));
            }
        }
        Ok(results)
    }
    
    async fn execute_proposal(&self, proposal: Proposal) -> Result<Value, VMError> {
        // Store the proposal
        self.state.proposals.insert(proposal.title.clone(), proposal.clone());
        
        // In a real implementation, this would:
        // 1. Validate the proposal
        // 2. Set up voting
        // 3. Execute the proposal if approved
        // For now, we'll just execute it immediately
        
        let results = self.execute_steps(&proposal.execution).await?;
        
        Ok(Value::Array(results))
    }
    
    async fn execute_asset_definition(&self, asset: Asset) -> Result<Value, VMError> {
        // Store the asset definition
        self.state.assets.insert(asset.name.clone(), asset);
        Ok(Value::Boolean(true))
    }
    
    async fn execute_role_definition(&self, role: Role) -> Result<Value, VMError> {
        // Store the role definition
        self.state.roles.insert(role.name.clone(), role);
        Ok(Value::Boolean(true))
    }
    
    async fn execute_membership_definition(&self, membership: Membership) -> Result<Value, VMError> {
        // Store the membership configuration
        self.state.memberships.insert(membership.name.clone(), membership);
        Ok(Value::Boolean(true))
    }
    
    async fn execute_federation_definition(&self, federation: Federation) -> Result<Value, VMError> {
        // Store the federation definition
        self.state.federations.insert(federation.name.clone(), federation);
        Ok(Value::Boolean(true))
    }
    
    async fn execute_credit_system_definition(&self, credit_system: CreditSystem) -> Result<Value, VMError> {
        // Store the credit system configuration
        self.state.credit_systems.insert(credit_system.name.clone(), credit_system);
        Ok(Value::Boolean(true))
    }
    
    // Member management
    pub async fn add_member(&self, member: Member) -> Result<(), VMError> {
        if self.state.members.contains_key(&member.id) {
            return Err(VMError::ValidationError(format!("Member with ID {} already exists", member.id)));
        }
        
        self.state.members.insert(member.id.clone(), member);
        Ok(())
    }
    
    pub async fn update_member(&self, member: Member) -> Result<(), VMError> {
        if !self.state.members.contains_key(&member.id) {
            return Err(VMError::ValidationError(format!("Member with ID {} does not exist", member.id)));
        }
        
        self.state.members.insert(member.id.clone(), member);
        Ok(())
    }
    
    pub async fn remove_member(&self, member_id: &str) -> Result<(), VMError> {
        if !self.state.members.contains_key(member_id) {
            return Err(VMError::ValidationError(format!("Member with ID {} does not exist", member_id)));
        }
        
        self.state.members.remove(member_id);
        Ok(())
    }
    
    // Voting
    pub async fn cast_vote(&self, vote: Vote) -> Result<(), VMError> {
        let proposal_id = vote.proposal_id.clone();
        
        // Check if proposal exists
        if !self.state.proposals.contains_key(&proposal_id) {
            return Err(VMError::ValidationError(format!("Proposal with ID {} does not exist", proposal_id)));
        }
        
        // Get or create the votes map for this proposal
        let mut votes = self.state.votes.entry(proposal_id.clone()).or_insert_with(HashMap::new);
        
        // Store the vote
        votes.insert(vote.member_id.clone(), vote);
        
        // Check if voting is complete and execute proposal if needed
        self.check_proposal_status(&proposal_id).await?;
        
        Ok(())
    }
    
    async fn check_proposal_status(&self, proposal_id: &str) -> Result<(), VMError> {
        // Get the proposal
        let proposal = if let Some(p) = self.state.proposals.get(proposal_id) {
            p.clone()
        } else {
            return Err(VMError::ValidationError(format!("Proposal with ID {} does not exist", proposal_id)));
        };
        
        // Get the votes
        let votes = if let Some(v) = self.state.votes.get(proposal_id) {
            v.clone()
        } else {
            // No votes yet
            return Ok(());
        };
        
        // Count votes and check quorum
        let total_members = self.state.members.len() as f64;
        let vote_count = votes.len() as f64;
        let quorum_percentage = proposal.quorum;
        
        if vote_count / total_members * 100.0 < quorum_percentage {
            // Quorum not reached
            return Ok(());
        }
        
        // Tally votes based on voting method
        let approved = match proposal.voting_method {
            VotingMethod::Majority => {
                let mut yes_votes = 0;
                let mut no_votes = 0;
                
                for (_, vote) in votes.iter() {
                    match vote.vote {
                        VoteValue::Yes => yes_votes += 1,
                        VoteValue::No => no_votes += 1,
                        _ => {}
                    }
                }
                
                yes_votes > no_votes
            },
            VotingMethod::Consensus => {
                // For consensus, we require a high threshold (e.g., 80% yes)
                let threshold = proposal.threshold.unwrap_or(80.0);
                let mut yes_votes = 0;
                
                for (_, vote) in votes.iter() {
                    if let VoteValue::Yes = vote.vote {
                        yes_votes += 1;
                    }
                }
                
                (yes_votes as f64 / vote_count * 100.0) >= threshold
            },
            VotingMethod::RankedChoice => {
                // Simplified ranked choice implementation
                // In a real system, this would be more complex
                true
            },
            VotingMethod::Quadratic => {
                // Simplified quadratic voting implementation
                // In a real system, this would involve more complex weight calculations
                true
            },
            VotingMethod::SingleChoice => {
                // Simple majority for single choice
                let mut yes_votes = 0;
                let mut no_votes = 0;
                
                for (_, vote) in votes.iter() {
                    match vote.vote {
                        VoteValue::Yes => yes_votes += 1,
                        VoteValue::No => no_votes += 1,
                        _ => {}
                    }
                }
                
                yes_votes > no_votes
            },
            VotingMethod::Custom(_) => {
                // Would implement custom voting logic based on the parameters
                // For now, default to true
                true
            }
        };
        
        // Execute the proposal if approved
        if approved {
            let _ = self.execute_steps(&proposal.execution).await?;
        } else if let Some(rejection_steps) = &proposal.rejection {
            let _ = self.execute_steps(rejection_steps).await?;
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use icn_dsl::{ExecutionStep, VotingMethod};
    
    #[tokio::test]
    async fn test_execute_proposal() {
        let vm = VM::new();
        
        let proposal = Proposal {
            title: "Test Proposal".to_string(),
            description: "A test proposal".to_string(),
            quorum: 60.0,
            threshold: Some(50.0),
            voting_method: VotingMethod::Majority,
            required_role: None,
            voting_period: None,
            category: None,
            tags: None,
            execution: vec![
                ExecutionStep {
                    function: "allocateFunds".to_string(),
                    args: vec![
                        Value::String("Education".to_string()),
                        Value::Number(500.0),
                    ],
                },
                ExecutionStep {
                    function: "notifyMembers".to_string(),
                    args: vec![Value::String("Proposal executed".to_string())],
                },
            ],
            rejection: None,
        };
        
        let result = vm.execute(ASTNode::Proposal(proposal)).await.unwrap();
        
        match result {
            Value::Array(results) => {
                assert_eq!(results.len(), 2);
                assert!(matches!(results[0], Value::Boolean(true)));
                assert!(matches!(results[1], Value::Boolean(true)));
            }
            _ => panic!("Expected array result"),
        }
    }
    
    #[tokio::test]
    async fn test_execute_membership() {
        let vm = VM::new();
        
        let membership = Membership {
            name: "DefaultMembership".to_string(),
            onboarding: OnboardingMethod::ApprovalVote,
            default_role: Some("Member".to_string()),
            max_members: Some(100),
            voting_rights: Some(true),
            credentials: None,
            attributes: HashMap::new(),
        };
        
        let result = vm.execute(ASTNode::Membership(membership)).await.unwrap();
        assert!(matches!(result, Value::Boolean(true)));
        
        // Verify it was stored in state
        assert!(vm.state.memberships.contains_key("DefaultMembership"));
    }
    
    #[tokio::test]
    async fn test_execute_role() {
        let vm = VM::new();
        
        let role = Role {
            name: "Admin".to_string(),
            description: Some("Administrator role".to_string()),
            permissions: vec!["create_proposal".to_string(), "manage_members".to_string()],
            parent_role: None,
            max_members: Some(5),
            assignable_by: None,
            attributes: HashMap::new(),
        };
        
        let result = vm.execute(ASTNode::Role(role)).await.unwrap();
        assert!(matches!(result, Value::Boolean(true)));
        
        // Verify it was stored in state
        assert!(vm.state.roles.contains_key("Admin"));
    }
} 