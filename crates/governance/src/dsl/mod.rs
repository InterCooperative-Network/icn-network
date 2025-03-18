/// Domain-Specific Language (DSL) for ICN Governance
///
/// This module re-exports and integrates the DSL functionality from the CLI crate
/// into the governance system, allowing for programmatic definition and execution
/// of governance rules, proposals, and voting.

// Re-export core DSL functionality
pub use icn_cli::dsl::{
    DslSystem, 
    DslEvent, 
    VoteType,
    parse, 
    parse_script,
    create_default_system
};

// Re-export parser functionality
pub use icn_cli::dsl::parser::{
    Parser,
    ast::{
        Program, 
        Statement, 
        ProposalStatement,
        AssetStatement, 
        TransactionStatement,
        FederationStatement, 
        VoteStatement,
        RoleStatement, 
        PermissionStatement,
        LogStatement, 
        Expression
    }
};

// Re-export VM functionality
pub use icn_cli::dsl::vm::VirtualMachine;

// Re-export standard library functions
pub use icn_cli::dsl::stdlib;

// Re-export integration helpers
pub use icn_cli::dsl::integration;

use crate::ProposalManager;
use std::sync::Arc;
use tokio::sync::mpsc;
use anyhow::Result;

/// DSL Manager for the governance system
pub struct GovernanceDslManager {
    /// The DSL system
    dsl_system: DslSystem,
    /// Event receiver from the DSL system
    event_receiver: mpsc::Receiver<DslEvent>,
    /// Reference to the proposal manager
    proposal_manager: Arc<ProposalManager>,
}

impl GovernanceDslManager {
    /// Create a new DSL manager for governance
    pub async fn new(proposal_manager: Arc<ProposalManager>) -> Self {
        // Create the default DSL system
        let (dsl_system, event_receiver) = create_default_system().await;
        
        Self {
            dsl_system,
            event_receiver,
            proposal_manager,
        }
    }
    
    /// Execute a DSL script
    pub async fn execute_script(&self, script: &str) -> Result<()> {
        self.dsl_system.execute_script(script).await
    }
    
    /// Execute a DSL script from a file
    pub async fn execute_script_file(&self, path: &str) -> Result<()> {
        self.dsl_system.execute_script_file(path).await
    }
    
    /// Start processing DSL events
    pub async fn start_event_processing(&mut self) -> Result<()> {
        // Process events from the DSL system
        while let Some(event) = self.event_receiver.recv().await {
            match event {
                DslEvent::ProposalCreated { id, title, description } => {
                    // Create a new proposal in the governance system
                    self.proposal_manager.create_proposal(&id, &title, &description).await?;
                },
                DslEvent::VoteCast { proposal_id, voter_id, vote } => {
                    // Record a vote on a proposal
                    match vote {
                        VoteType::Yes => {
                            self.proposal_manager.cast_vote(&proposal_id, &voter_id, true).await?;
                        },
                        VoteType::No => {
                            self.proposal_manager.cast_vote(&proposal_id, &voter_id, false).await?;
                        },
                        // Handle other vote types as needed
                        _ => {}
                    }
                },
                DslEvent::ProposalExecuted { id, result } => {
                    // Mark a proposal as executed
                    if result {
                        self.proposal_manager.mark_proposal_executed(&id).await?;
                    }
                },
                // Handle other event types as needed
                _ => {}
            }
        }
        
        Ok(())
    }
} 