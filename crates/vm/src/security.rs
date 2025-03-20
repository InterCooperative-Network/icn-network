use thiserror::Error;
use icn_common::types::{OperationContext, Value, DID};
use crate::operations::{Operation, OperationType};
use crate::VMError;
use std::sync::Arc;
use std::collections::HashMap;
use dashmap::DashMap;

#[derive(Error, Debug)]
pub enum SecurityError {
    #[error("Unauthorized: {0}")]
    Unauthorized(String),
    
    #[error("Forbidden: {0}")]
    Forbidden(String),
    
    #[error("Quota exceeded: {0}")]
    QuotaExceeded(String),
    
    #[error("Resource limit exceeded: {0}")]
    ResourceLimitExceeded(String),
    
    #[error("Invalid signature: {0}")]
    InvalidSignature(String),
}

/// Security sandbox for the VM
pub struct SecuritySandbox {
    /// Operation permissions by type
    op_permissions: DashMap<OperationType, Vec<String>>,
    /// Entity permissions by type
    entity_permissions: DashMap<String, Vec<String>>,
    /// Admin roles
    admin_roles: Vec<String>,
    /// Role assignments
    roles: DashMap<String, Vec<String>>,
}

impl SecuritySandbox {
    /// Create a new security sandbox
    pub fn new() -> Self {
        let mut sandbox = Self {
            op_permissions: DashMap::new(),
            entity_permissions: DashMap::new(),
            admin_roles: vec!["admin".to_string(), "system".to_string()],
            roles: DashMap::new(),
        };
        
        // Set up default permissions
        sandbox.setup_default_permissions();
        
        sandbox
    }
    
    /// Set up default permissions
    fn setup_default_permissions(&self) {
        // Governance permissions
        self.op_permissions.insert(
            OperationType::Governance,
            vec![
                "governance_admin".to_string(),
                "federation_admin".to_string(),
                "dao_member".to_string(),
            ],
        );
        
        // Identity permissions
        self.op_permissions.insert(
            OperationType::Identity,
            vec![
                "identity_admin".to_string(),
                "federation_admin".to_string(),
            ],
        );
        
        // Economic permissions
        self.op_permissions.insert(
            OperationType::Economic,
            vec![
                "economic_admin".to_string(),
                "federation_admin".to_string(),
                "dao_treasurer".to_string(),
            ],
        );
        
        // Network permissions
        self.op_permissions.insert(
            OperationType::Network,
            vec![
                "network_admin".to_string(),
                "federation_admin".to_string(),
            ],
        );
        
        // Resource permissions
        self.op_permissions.insert(
            OperationType::Resource,
            vec![
                "resource_admin".to_string(),
                "federation_admin".to_string(),
            ],
        );
        
        // General permissions
        self.op_permissions.insert(
            OperationType::General,
            vec![
                "admin".to_string(),
                "system".to_string(),
            ],
        );
        
        // Entity permissions
        self.entity_permissions.insert(
            "proposal".to_string(),
            vec![
                "governance_admin".to_string(),
                "dao_member".to_string(),
            ],
        );
        
        self.entity_permissions.insert(
            "vote".to_string(),
            vec![
                "governance_admin".to_string(),
                "dao_member".to_string(),
            ],
        );
        
        self.entity_permissions.insert(
            "member".to_string(),
            vec![
                "identity_admin".to_string(),
                "federation_admin".to_string(),
            ],
        );
    }
    
    /// Validate an operation
    pub fn validate_operation(&self, operation: &Operation, context: &OperationContext) -> Result<()> {
        // System always has access
        if context.caller.as_str() == "system" {
            return Ok(());
        }
        
        // Get the operation type
        let op_type = operation.operation_type();
        
        // Check if the caller has permission for this operation type
        if !self.has_permission_for_op_type(&context.caller, &op_type) {
            return Err(VMError::PermissionError(format!(
                "Caller {} does not have permission for operation type {:?}",
                context.caller.as_str(), op_type
            )));
        }
        
        // Validate specific operations
        match operation {
            Operation::ExecuteProposal(proposal_id) => {
                self.validate_proposal_execution(proposal_id, context)
            },
            Operation::ValidateVote(vote_id) => {
                // Anyone can validate a vote
                Ok(())
            },
            Operation::CreateEntity(entity_type, _) => {
                self.validate_entity_operation(&context.caller, entity_type, "create")
            },
            Operation::UpdateEntity(entity_type, entity_id, _) => {
                self.validate_entity_operation(&context.caller, entity_type, "update")
            },
            Operation::DeleteEntity(entity_type, entity_id) => {
                self.validate_entity_operation(&context.caller, entity_type, "delete")
            },
            _ => {
                // Default validation just based on operation type
                Ok(())
            },
        }
    }
    
    /// Check if a caller has permission for an operation type
    fn has_permission_for_op_type(&self, caller: &DID, op_type: &OperationType) -> bool {
        // Admin roles always have permission
        if self.has_admin_role(caller) {
            return true;
        }
        
        // Check if the caller has a role with permission for this operation type
        if let Some(permissions) = self.op_permissions.get(op_type) {
            let caller_roles = self.get_roles(caller);
            for role in &caller_roles {
                if permissions.contains(role) {
                    return true;
                }
            }
        }
        
        false
    }
    
    /// Validate a proposal execution
    fn validate_proposal_execution(&self, proposal_id: &str, context: &OperationContext) -> Result<()> {
        // In a real implementation, this would check if the proposal is approved
        // and if the caller has permission to execute it
        // For now, just validate based on the operation type
        Ok(())
    }
    
    /// Validate an entity operation
    fn validate_entity_operation(&self, caller: &DID, entity_type: &str, operation: &str) -> Result<()> {
        // Admin roles always have permission
        if self.has_admin_role(caller) {
            return Ok(());
        }
        
        // Check if the caller has a role with permission for this entity type
        if let Some(permissions) = self.entity_permissions.get(entity_type) {
            let caller_roles = self.get_roles(caller);
            for role in &caller_roles {
                if permissions.contains(role) {
                    return Ok(());
                }
            }
        }
        
        Err(VMError::PermissionError(format!(
            "Caller {} does not have permission to {} entity type {}",
            caller.as_str(), operation, entity_type
        )))
    }
    
    /// Check if a caller has an admin role
    fn has_admin_role(&self, caller: &DID) -> bool {
        let caller_roles = self.get_roles(caller);
        for role in &caller_roles {
            if self.admin_roles.contains(role) {
                return true;
            }
        }
        false
    }
    
    /// Get the roles for a caller
    fn get_roles(&self, caller: &DID) -> Vec<String> {
        // In a real implementation, this would look up the roles from some storage
        // For now, just return a default set of roles
        if let Some(roles) = self.roles.get(caller.as_str()) {
            roles.clone()
        } else {
            // Default role
            vec!["dao_member".to_string()]
        }
    }
    
    /// Assign a role to a caller
    pub fn assign_role(&self, caller: &DID, role: &str) {
        let mut roles = self.get_roles(caller);
        if !roles.contains(&role.to_string()) {
            roles.push(role.to_string());
            self.roles.insert(caller.as_str().to_string(), roles);
        }
    }
    
    /// Remove a role from a caller
    pub fn remove_role(&self, caller: &DID, role: &str) {
        let roles = self.get_roles(caller);
        let roles = roles.into_iter()
            .filter(|r| r != role)
            .collect();
    /// Check if the caller has permission to execute the operation
    fn check_permissions(
        &self,
        operation: &Operation,
        context: &OperationContext,
    ) -> Result<(), VMError> {
        // This is just a placeholder implementation
        // In a real system, this would check against a permission system
        
        // For now, we'll just allow everything
        Ok(())
    }
    
    /// Check if the operation exceeds resource limits
    fn check_resource_limits(
        &self,
        operation: &Operation,
        context: &OperationContext,
    ) -> Result<(), VMError> {
        // This is just a placeholder implementation
        // In a real system, this would check against resource quotas
        
        // For now, we'll just allow everything
        Ok(())
    }
    
    /// Verify the signature of the operation
    fn verify_signature(
        &self,
        operation: &Operation,
        context: &OperationContext,
    ) -> Result<(), VMError> {
        // This is just a placeholder implementation
        // In a real system, this would verify cryptographic signatures
        
        // For now, we'll just allow everything
        Ok(())
    }
    
    /// Enable or disable permission checking
    pub fn set_permission_checking(&mut self, enabled: bool) {
        self.permission_checking = enabled;
    }
    
    /// Enable or disable resource limiting
    pub fn set_resource_limiting(&mut self, enabled: bool) {
        self.resource_limiting = enabled;
    }
    
    /// Enable or disable signature verification
    pub fn set_signature_verification(&mut self, enabled: bool) {
        self.signature_verification = enabled;
    }
} 