use anyhow::Result;
use dashmap::DashMap;
use icn_dsl::{ASTNode, Value, Proposal, Asset, Role};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

#[derive(Debug, Error)]
pub enum VMError {
    #[error("Execution error: {0}")]
    ExecutionError(String),
    #[error("State error: {0}")]
    StateError(String),
    #[error("Permission error: {0}")]
    PermissionError(String),
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
    /// General key-value store for other state
    store: DashMap<String, Value>,
}

impl VMState {
    pub fn new() -> Self {
        Self {
            proposals: DashMap::new(),
            assets: DashMap::new(),
            roles: DashMap::new(),
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
    }
    
    /// Execute a parsed AST node
    pub async fn execute(&self, node: ASTNode) -> Result<Value, VMError> {
        match node {
            ASTNode::Proposal(proposal) => self.execute_proposal(proposal).await,
            ASTNode::Asset(asset) => self.execute_asset_definition(asset).await,
            ASTNode::Role(role) => self.execute_role_definition(role).await,
        }
    }
    
    async fn execute_proposal(&self, proposal: Proposal) -> Result<Value, VMError> {
        // Store the proposal
        self.state.proposals.insert(proposal.title.clone(), proposal.clone());
        
        // In a real implementation, this would:
        // 1. Validate the proposal
        // 2. Set up voting
        // 3. Execute the proposal if approved
        // For now, we'll just execute it immediately
        
        let mut results = Vec::new();
        for step in proposal.execution {
            if let Some(func) = self.functions.get(&step.function) {
                results.push(func(step.args)?);
            } else {
                return Err(VMError::ExecutionError(format!(
                    "Unknown function: {}",
                    step.function
                )));
            }
        }
        
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
            voting_method: VotingMethod::Majority,
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
} 