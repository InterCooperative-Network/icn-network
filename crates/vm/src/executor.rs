//! Executor for VM operations

use std::sync::Arc;
use std::collections::HashMap;

use icn_common::types::{Value, DID, OperationContext};
use crate::error::{Result, VMError};
use crate::operations::Operation;
use crate::state::{StateAccess, StateManager};

/// Executor for VM operations
pub struct Executor {
    /// Custom function registry
    functions: HashMap<String, Box<dyn Fn(Vec<Value>) -> Result<Value> + Send + Sync>>,
}

impl Executor {
    /// Create a new executor
    pub fn new() -> Self {
        Self {
            functions: HashMap::new(),
        }
    }
    
    /// Register a custom function
    pub fn register_function<F>(&mut self, name: &str, function: F)
    where
        F: Fn(Vec<Value>) -> Result<Value> + Send + Sync + 'static,
    {
        self.functions.insert(name.to_string(), Box::new(function));
    }
    
    /// Execute an operation
    pub async fn execute(
        &self,
        operation: Operation,
        context: OperationContext,
        state: Arc<dyn StateAccess>,
    ) -> Result<Value> {
        match operation {
            Operation::ExecuteProposal(proposal_id) => {
                self.execute_proposal(&proposal_id, &context, state).await
            },
            Operation::ValidateVote(vote_id) => {
                self.validate_vote(&vote_id, &context, state).await
            },
            Operation::CreateEntity(entity_type, data) => {
                self.create_entity(&entity_type, data, &context, state).await
            },
            Operation::UpdateEntity(entity_type, entity_id, data) => {
                self.update_entity(&entity_type, &entity_id, data, &context, state).await
            },
            Operation::DeleteEntity(entity_type, entity_id) => {
                self.delete_entity(&entity_type, &entity_id, &context, state).await
            },
            Operation::Execute(function, args) => {
                self.execute_function(&function, args, &context, state).await
            },
            Operation::GetEntity(entity_type, entity_id) => {
                self.get_entity(&entity_type, &entity_id, &context, state).await
            },
            Operation::ListEntities(entity_type) => {
                self.list_entities(&entity_type, &context, state).await
            },
            Operation::Custom(operation_type, data) => {
                self.execute_custom(&operation_type, data, &context, state).await
            },
        }
    }
    
    /// Execute a proposal
    async fn execute_proposal(
        &self,
        proposal_id: &str,
        context: &OperationContext,
        state: Arc<dyn StateAccess>,
    ) -> Result<Value> {
        // Retrieve the proposal from state
        let proposal = state.get("proposals", proposal_id).await?
            .ok_or_else(|| VMError::NotFound(format!("Proposal not found: {}", proposal_id)))?;
        
        // Mark the proposal as executed
        // In a real implementation, this would actually execute the proposal actions
        let mut proposal_map = if let Value::Object(map) = proposal {
            map
        } else {
            return Err(VMError::ValidationError("Invalid proposal format".to_string()));
        };
        
        proposal_map.insert("status".to_string(), Value::String("executed".to_string()));
        proposal_map.insert("executed_at".to_string(), Value::Int(context.timestamp as i64));
        proposal_map.insert("executed_by".to_string(), Value::String(context.caller.as_str().to_string()));
        
        state.put("proposals", proposal_id, Value::Object(proposal_map)).await?;
        
        Ok(Value::Bool(true))
    }
    
    /// Validate a vote
    async fn validate_vote(
        &self,
        vote_id: &str,
        context: &OperationContext,
        state: Arc<dyn StateAccess>,
    ) -> Result<Value> {
        // Retrieve the vote from state
        let vote = state.get("votes", vote_id).await?
            .ok_or_else(|| VMError::NotFound(format!("Vote not found: {}", vote_id)))?;
        
        // In a real implementation, this would validate the vote against governance rules
        // For now, just return true
        Ok(Value::Bool(true))
    }
    
    /// Create an entity
    async fn create_entity(
        &self,
        entity_type: &str,
        data: Value,
        context: &OperationContext,
        state: Arc<dyn StateAccess>,
    ) -> Result<Value> {
        // Generate a unique ID for the entity
        let entity_id = format!("{}-{}", entity_type, uuid::Uuid::new_v4());
        
        // Store the entity in state
        state.put(entity_type, &entity_id, data).await?;
        
        Ok(Value::String(entity_id))
    }
    
    /// Update an entity
    async fn update_entity(
        &self,
        entity_type: &str,
        entity_id: &str,
        data: Value,
        _context: &OperationContext,
        state: Arc<dyn StateAccess>,
    ) -> Result<Value> {
        // Check if the entity exists
        if state.get(entity_type, entity_id).await?.is_none() {
            return Err(VMError::NotFound(format!("Entity not found: {}/{}", entity_type, entity_id)));
        }
        
        // Update the entity
        state.put(entity_type, entity_id, data).await?;
        
        Ok(Value::Bool(true))
    }
    
    /// Delete an entity
    async fn delete_entity(
        &self,
        entity_type: &str,
        entity_id: &str,
        _context: &OperationContext,
        state: Arc<dyn StateAccess>,
    ) -> Result<Value> {
        // Check if the entity exists
        if state.get(entity_type, entity_id).await?.is_none() {
            return Err(VMError::NotFound(format!("Entity not found: {}/{}", entity_type, entity_id)));
        }
        
        // Delete the entity
        state.delete(entity_type, entity_id).await?;
        
        Ok(Value::Bool(true))
    }
    
    /// Execute a custom function
    async fn execute_function(
        &self,
        function: &str,
        args: Vec<Value>,
        _context: &OperationContext,
        _state: Arc<dyn StateAccess>,
    ) -> Result<Value> {
        // Look up the function
        if let Some(func) = self.functions.get(function) {
            func(args)
        } else {
            Err(VMError::NotFound(format!("Function not found: {}", function)))
        }
    }
    
    /// Get an entity
    async fn get_entity(
        &self,
        entity_type: &str,
        entity_id: &str,
        _context: &OperationContext,
        state: Arc<dyn StateAccess>,
    ) -> Result<Value> {
        // Retrieve the entity
        let entity = state.get(entity_type, entity_id).await?
            .ok_or_else(|| VMError::NotFound(format!("Entity not found: {}/{}", entity_type, entity_id)))?;
        
        Ok(entity)
    }
    
    /// List all entities of a type
    async fn list_entities(
        &self,
        entity_type: &str,
        _context: &OperationContext,
        state: Arc<dyn StateAccess>,
    ) -> Result<Value> {
        // Get all entity IDs of this type
        let ids = state.list(entity_type).await?;
        
        // Convert to Value::Array
        let ids_value = ids.into_iter()
            .map(Value::String)
            .collect();
        
        Ok(Value::Array(ids_value))
    }
    
    /// Execute a custom operation
    async fn execute_custom(
        &self,
        operation_type: &str,
        data: Value,
        _context: &OperationContext,
        _state: Arc<dyn StateAccess>,
    ) -> Result<Value> {
        // In a real implementation, this would dispatch to plugin handlers
        // For now, just return an error
        Err(VMError::ValidationError(format!("Unsupported custom operation: {}", operation_type)))
    }
} 