//! Virtual Machine for the ICN Network
//!
//! This module provides the core VM functionality for executing operations
//! in the ICN Network, including governance, economic, and reputation operations.

pub mod executor;
pub mod security;
pub mod builtins;
pub mod storage;
pub mod operations;
pub mod state;
pub mod error;
pub mod util;

use std::sync::Arc;
use std::collections::HashMap;

use icn_common::types::{Value, DID, OperationContext};
use error::{Result, VMError};
use operations::Operation;
use executor::Executor;
use security::SecuritySandbox;
use state::StateManager;

pub use state::StateAccess;
pub use error::VMError;
pub use operations::Operation;

/// The Virtual Machine for the ICN Network
pub struct VM {
    /// The executor for running operations
    executor: Arc<Executor>,
    /// The security sandbox
    security: Arc<SecuritySandbox>,
    /// The state manager
    state: Arc<StateManager>,
}

impl VM {
    /// Create a new VM instance
    pub fn new(
        executor: Arc<Executor>, 
        security: Arc<SecuritySandbox>,
        state: Arc<StateManager>,
    ) -> Self {
        Self {
            executor,
            security,
            state,
        }
    }

    /// Execute an operation in the VM
    pub async fn execute(&self, operation: Operation, context: OperationContext) -> Result<Value> {
        // Validate operation through security sandbox
        self.security.validate_operation(&operation, &context)
            .map_err(|e| VMError::SecurityError(e.to_string()))?;

        // Execute the operation
        self.executor.execute(operation, context, self.state.clone()).await
    }

    /// Access the state manager
    pub fn state(&self) -> Arc<StateManager> {
        self.state.clone()
    }

    /// Access the security sandbox
    pub fn security(&self) -> Arc<SecuritySandbox> {
        self.security.clone()
    }
} 