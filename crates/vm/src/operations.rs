//! VM operations for the ICN Network

use icn_common::types::Value;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Operation type categorization
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum OperationType {
    /// Governance operations
    Governance,
    /// Economic operations
    Economic,
    /// Identity operations
    Identity,
    /// Resource operations
    Resource,
    /// Network operations
    Network,
    /// General operations
    General,
}

/// VM operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Operation {
    /// Execute a governance proposal
    ExecuteProposal(String),
    
    /// Validate a vote
    ValidateVote(String),
    
    /// Create an entity
    CreateEntity(String, Value),
    
    /// Update an entity
    UpdateEntity(String, String, Value),
    
    /// Delete an entity
    DeleteEntity(String, String),
    
    /// Execute a custom function
    Execute(String, Vec<Value>),
    
    /// Get an entity
    GetEntity(String, String),
    
    /// List entities
    ListEntities(String),
    
    /// Custom operation
    Custom(String, Value),
}

impl Operation {
    /// Get the type of this operation
    pub fn operation_type(&self) -> OperationType {
        match self {
            Operation::ExecuteProposal(..) => OperationType::Governance,
            Operation::ValidateVote(..) => OperationType::Governance,
            Operation::CreateEntity(..) => OperationType::Identity,
            Operation::UpdateEntity(..) => OperationType::Identity,
            Operation::DeleteEntity(..) => OperationType::Identity,
            Operation::Execute(..) => OperationType::General,
            Operation::GetEntity(..) => OperationType::Identity,
            Operation::ListEntities(..) => OperationType::Identity,
            Operation::Custom(..) => OperationType::General,
        }
    }
} 