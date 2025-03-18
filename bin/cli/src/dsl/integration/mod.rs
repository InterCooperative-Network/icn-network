/// DSL Integration Module
///
/// This module provides integration points between the DSL system and existing
/// ICN components such as governance, networking, and storage.

pub mod network_integration;
pub mod governance_integration;

use crate::dsl::{DslEvent, DslSystem, VoteType};
use crate::governance::{GovernanceService, ProposalStatus, ProposalType};
use crate::networking::{NetworkManager, FederationConfig};
use crate::storage::StorageService;
use anyhow::{Result, anyhow, Context};
use tokio::sync::{mpsc, oneshot};
use std::sync::Arc;

/// DSL Integration Manager
///
/// This struct manages the integration between DSL and other ICN components.
pub struct DslIntegrationManager {
    /// DSL System
    dsl_system: DslSystem,
    /// Event receiver for DSL events
    event_rx: mpsc::Receiver<DslEvent>,
    /// Network Manager
    network_manager: NetworkManager,
    /// Governance Service
    governance_service: Arc<GovernanceService>,
    /// Storage Service
    storage_service: Arc<StorageService>,
}

impl DslIntegrationManager {
    /// Create a new DSL Integration Manager
    pub async fn new(
        network_manager: NetworkManager,
        governance_service: Arc<GovernanceService>,
        storage_service: Arc<StorageService>,
    ) -> Result<Self> {
        let (dsl_system, event_rx) = crate::dsl::create_default_system().await;
        
        Ok(Self {
            dsl_system,
            event_rx,
            network_manager,
            governance_service,
            storage_service,
        })
    }
    
    /// Start the event handler
    pub async fn start(&mut self) -> Result<()> {
        // Handle DSL events
        while let Some(event) = self.event_rx.recv().await {
            self.handle_event(event).await?;
        }
        
        Ok(())
    }
    
    /// Handle a DSL event
    async fn handle_event(&self, event: DslEvent) -> Result<()> {
        match event {
            DslEvent::ProposalCreated { id, title, description } => {
                // Create a proposal in the governance system
                self.governance_service.create_proposal(
                    &title,
                    &description,
                    ProposalType::Policy,
                    "default", // federation
                    "dsl_system", // proposer
                    51, // quorum
                    51, // approval
                    None, // content_file
                ).await.context("Failed to create proposal")?;
                
                println!("Proposal created from DSL: {}", id);
            },
            DslEvent::VoteCast { proposal_id, voter_id, vote } => {
                // Convert DSL vote to governance vote
                let vote_str = match vote {
                    VoteType::Yes => "yes",
                    VoteType::No => "no",
                    VoteType::Abstain => "abstain",
                    VoteType::RankedChoice(_) => {
                        return Err(anyhow!("Ranked choice voting not implemented yet"));
                    }
                };
                
                // Cast a vote in the governance system
                self.governance_service.vote(
                    &proposal_id,
                    &voter_id,
                    vote_str,
                    None, // comment
                    1.0, // weight
                    "default", // federation
                ).await.context("Failed to cast vote")?;
                
                println!("Vote cast from DSL: {} by {} on proposal {}", vote_str, voter_id, proposal_id);
            },
            DslEvent::ProposalExecuted { id, result } => {
                if result {
                    // Execute the proposal in the governance system
                    self.governance_service.execute_proposal(
                        &id,
                        "default", // federation
                    ).await.context("Failed to execute proposal")?;
                    
                    println!("Proposal executed from DSL: {}", id);
                } else {
                    // Update proposal status to rejected
                    self.governance_service.update_status(
                        &id,
                        "rejected",
                        "default", // federation
                    ).await.context("Failed to update proposal status")?;
                    
                    println!("Proposal rejected from DSL: {}", id);
                }
            },
            DslEvent::Transaction { from, to, amount, asset_type } => {
                // TODO: Implement transaction handling
                println!("Transaction from DSL: {} {} from {} to {}", amount, asset_type, from, to);
            },
            DslEvent::Log(message) => {
                println!("DSL Log: {}", message);
            },
            DslEvent::Error(error) => {
                println!("DSL Error: {}", error);
            },
        }
        
        Ok(())
    }
    
    /// Execute a DSL script
    pub async fn execute_script(&self, script: &str) -> Result<()> {
        self.dsl_system.execute_script(script).await
    }
    
    /// Execute a DSL script from a file
    pub async fn execute_script_file(&self, path: &str) -> Result<()> {
        self.dsl_system.execute_script_file(path).await
    }
}

/// Initialize the DSL integration with the CLI
pub async fn initialize_dsl_cli() -> Result<()> {
    // This will be called from main.rs to set up the DSL CLI commands
    Ok(())
}
