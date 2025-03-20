use std::sync::Arc;
use std::collections::HashMap;
use icn_vm::{VM, Operation, ExecutionResult, VMError, StateAccess};
use icn_common::types::{Value, OperationContext, DID};
use crate::{Proposal, Vote, GovernanceError, GovernanceResult};

/// GovernanceVMIntegration provides an interface to execute governance operations
/// through the virtual machine
pub struct GovernanceVMIntegration {
    /// Reference to the virtual machine
    vm: Arc<VM>,
}

impl GovernanceVMIntegration {
    /// Create a new governance VM integration
    pub fn new(vm: Arc<VM>) -> Self {
        Self { vm }
    }
    
    /// Execute a proposal through the VM
    pub async fn execute_proposal(&self, proposal_id: &str) -> GovernanceResult<()> {
        let operation = Operation::ExecuteProposal(proposal_id.to_string());
        let context = OperationContext {
            caller: DID::new("system"),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            metadata: HashMap::new(),
        };

        self.vm.execute(operation, context)
            .await
            .map_err(|e| GovernanceError::VMExecutionError(e.to_string()))?;

        Ok(())
    }
    
    /// Validate a vote through the VM
    pub async fn validate_vote(&self, vote: &Vote) -> GovernanceResult<bool> {
        let operation = Operation::ValidateVote(vote.id.clone());
        let context = OperationContext {
            caller: vote.voter_id.clone(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            metadata: HashMap::new(),
        };

        let result = self.vm.execute(operation, context)
            .await
            .map_err(|e| GovernanceError::VMExecutionError(e.to_string()))?;

        // The result is expected to be a boolean indicating if the vote is valid
        match result {
            Value::Bool(valid) => Ok(valid),
            _ => Err(GovernanceError::InternalError("Unexpected return type from VM".to_string())),
        }
    }
    
    /// Create a new governance entity through the VM
    pub async fn create_entity(&self, entity_type: &str, data: Value) -> GovernanceResult<String> {
        let operation = Operation::CreateEntity(entity_type.to_string(), data);
        let context = OperationContext {
            caller: DID::new("system"),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            metadata: HashMap::new(),
        };

        let result = self.vm.execute(operation, context)
            .await
            .map_err(|e| GovernanceError::VMExecutionError(e.to_string()))?;

        // The result is expected to be a string ID of the created entity
        match result {
            Value::String(id) => Ok(id),
            _ => Err(GovernanceError::InternalError("Unexpected return type from VM".to_string())),
        }
    }
    
    /// Update an existing governance entity through the VM
    pub async fn update_entity(&self, entity_type: &str, entity_id: &str, data: Value) -> GovernanceResult<()> {
        let operation = Operation::UpdateEntity(entity_type.to_string(), entity_id.to_string(), data);
        let context = OperationContext {
            caller: DID::new("system"),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            metadata: HashMap::new(),
        };

        self.vm.execute(operation, context)
            .await
            .map_err(|e| GovernanceError::VMExecutionError(e.to_string()))?;

        Ok(())
    }
    
    /// Delete a governance entity through the VM
    pub async fn delete_entity(&self, entity_type: &str, entity_id: &str) -> GovernanceResult<()> {
        let operation = Operation::DeleteEntity(entity_type.to_string(), entity_id.to_string());
        let context = OperationContext {
            caller: DID::new("system"),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            metadata: HashMap::new(),
        };

        self.vm.execute(operation, context)
            .await
            .map_err(|e| GovernanceError::VMExecutionError(e.to_string()))?;

        Ok(())
    }
} 