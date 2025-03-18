//! Federation Governance Integration
//!
//! This module connects the federation networking capabilities with
//! the governance system, allowing cooperatives to participate in democratic
//! decision-making about network resources and federation policies.

use std::sync::Arc;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::governance::{GovernanceService, Proposal, ProposalStatus, ProposalType, Vote, MemberVote};
use super::network_manager::{NetworkManager, FederationNetworkConfig};

/// Types of federation network proposals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FederationNetworkProposalType {
    /// Add a new peer to the federation
    AddPeer { 
        peer_id: String, 
        peer_address: String,
    },
    /// Remove a peer from the federation
    RemovePeer { 
        peer_id: String,
    },
    /// Update federation network configuration
    UpdateConfig { 
        config: FederationNetworkConfig,
    },
    /// Enable cross-federation communication
    EnableCrossFederation { 
        target_federation: String,
    },
    /// Disable cross-federation communication
    DisableCrossFederation { 
        target_federation: String,
    },
    /// Enable WireGuard for the federation
    EnableWireGuard,
    /// Disable WireGuard for the federation
    DisableWireGuard,
    /// Add bootstrap peers to the federation
    AddBootstrapPeers { 
        peers: Vec<String>,
    },
}

/// Federation governance message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FederationGovernanceMessage {
    /// Publish a new proposal to the federation
    ProposalPublication { 
        proposal: Proposal,
    },
    /// Update to proposal status
    ProposalStatusUpdate { 
        proposal_id: String, 
        status: ProposalStatus,
    },
    /// Cast a vote on a proposal
    VoteCast { 
        proposal_id: String, 
        vote: MemberVote,
    },
    /// Notify that a proposal was executed
    ProposalExecution { 
        proposal_id: String, 
        result: bool,
        output: String,
    },
    /// Request missing governance data
    DataSyncRequest { 
        proposal_ids: Vec<String>,
    },
    /// Respond with governance data
    DataSyncResponse { 
        proposals: Vec<Proposal>,
    },
}

/// Service that integrates governance with federation networking
pub struct FederationGovernanceService {
    /// Reference to the network manager
    network_manager: Arc<NetworkManager>,
    /// Reference to the governance service
    governance_service: Arc<RwLock<GovernanceService>>,
    /// Currently active federation ID
    active_federation: String,
}

impl FederationGovernanceService {
    /// Create a new federation governance service
    pub async fn new(
        network_manager: Arc<NetworkManager>,
        governance_service: Arc<RwLock<GovernanceService>>,
    ) -> Result<Self> {
        // Get active federation from network manager
        let active_federation = network_manager.get_active_federation().await;
        
        let service = Self {
            network_manager,
            governance_service,
            active_federation,
        };
        
        // Initialize message handler to receive governance messages
        service.init_message_handler().await?;
        
        Ok(service)
    }
    
    /// Initialize the message handler for governance events
    async fn init_message_handler(&self) -> Result<()> {
        // TODO: Implement message handling for incoming governance messages
        // This would involve subscribing to governance-related messages
        // and processing them accordingly
        
        Ok(())
    }
    
    /// Create a network-related proposal
    pub async fn create_network_proposal(
        &self,
        title: &str,
        description: &str,
        proposal_type: FederationNetworkProposalType,
        proposer: &str,
    ) -> Result<String> {
        // Convert the network proposal type to JSON content
        let content = serde_json::to_value(&proposal_type)
            .map_err(|e| anyhow!("Failed to serialize proposal content: {}", e))?;
        
        // Lock the governance service
        let mut governance = self.governance_service.write().await;
        
        // Create standard governance proposal
        let proposal_id = governance.create_proposal(
            title,
            description,
            ProposalType::ConfigChange, // Use ConfigChange type for network proposals
            proposer,
            content,
            51, // Default quorum: 51%
            51, // Default approval: 51%
        ).await?;
        
        // Broadcast the proposal to all federation members
        let proposal = governance.get_proposal(&proposal_id)
            .ok_or_else(|| anyhow!("Failed to retrieve created proposal"))?;
        
        // Broadcast the new proposal to the federation
        self.broadcast_proposal(proposal).await?;
        
        Ok(proposal_id)
    }
    
    /// Broadcast a proposal to all federation peers
    async fn broadcast_proposal(&self, proposal: &Proposal) -> Result<()> {
        // Create governance message
        let message = FederationGovernanceMessage::ProposalPublication {
            proposal: proposal.clone(),
        };
        
        // Convert to JSON
        let message_json = serde_json::to_value(message)
            .map_err(|e| anyhow!("Failed to serialize governance message: {}", e))?;
        
        // Broadcast to current federation
        self.network_manager.broadcast_to_federation(
            &self.active_federation,
            "governance_proposal",
            message_json,
        ).await?;
        
        Ok(())
    }
    
    /// Cast a vote on a network proposal
    pub async fn cast_network_vote(
        &self,
        proposal_id: &str,
        member_id: &str,
        vote: Vote,
        comment: Option<String>,
        weight: f64,
    ) -> Result<()> {
        // Lock the governance service
        let mut governance = self.governance_service.write().await;
        
        // Cast the vote
        governance.cast_vote(
            proposal_id,
            member_id,
            vote.clone(),
            comment.clone(),
            weight,
        ).await?;
        
        // Get proposal to verify it's valid
        let proposal = governance.get_proposal(proposal_id)
            .ok_or_else(|| anyhow!("Proposal not found"))?;
        
        // Create vote message
        let timestamp = chrono::Utc::now().timestamp() as u64;
        let vote_message = MemberVote {
            member_id: member_id.to_string(),
            vote: vote.clone(),
            timestamp,
            comment,
            weight,
        };
        
        // Create governance message
        let message = FederationGovernanceMessage::VoteCast {
            proposal_id: proposal_id.to_string(),
            vote: vote_message,
        };
        
        // Convert to JSON
        let message_json = serde_json::to_value(message)
            .map_err(|e| anyhow!("Failed to serialize vote message: {}", e))?;
        
        // Broadcast to current federation
        self.network_manager.broadcast_to_federation(
            &self.active_federation,
            "governance_vote",
            message_json,
        ).await?;
        
        Ok(())
    }
    
    /// Execute an approved network proposal
    pub async fn execute_network_proposal(&self, proposal_id: &str) -> Result<()> {
        // Lock the governance service
        let mut governance = self.governance_service.write().await;
        
        // Get the proposal
        let proposal = governance.get_proposal(proposal_id)
            .ok_or_else(|| anyhow!("Proposal not found"))?;
        
        // Verify proposal is approved
        if proposal.status != ProposalStatus::Approved {
            return Err(anyhow!("Cannot execute proposal that is not approved"));
        }
        
        // Parse the network proposal content
        let network_proposal: FederationNetworkProposalType = serde_json::from_value(proposal.content.clone())
            .map_err(|e| anyhow!("Failed to parse network proposal content: {}", e))?;
        
        // Execute based on proposal type
        let result = match network_proposal {
            FederationNetworkProposalType::AddPeer { peer_id, peer_address } => {
                info!("Executing AddPeer proposal: {} at {}", peer_id, peer_address);
                match self.network_manager.connect(&peer_address).await {
                    Ok(_) => {
                        let output = format!("Successfully added peer {} at {}", peer_id, peer_address);
                        Ok(output)
                    },
                    Err(e) => Err(anyhow!("Failed to add peer: {}", e)),
                }
            },
            FederationNetworkProposalType::RemovePeer { peer_id } => {
                info!("Executing RemovePeer proposal: {}", peer_id);
                match self.network_manager.disconnect(&peer_id).await {
                    Ok(_) => {
                        let output = format!("Successfully removed peer {}", peer_id);
                        Ok(output)
                    },
                    Err(e) => Err(anyhow!("Failed to remove peer: {}", e)),
                }
            },
            FederationNetworkProposalType::UpdateConfig { config } => {
                info!("Executing UpdateConfig proposal for federation {}", self.active_federation);
                match self.network_manager.update_federation_config(&self.active_federation, config).await {
                    Ok(_) => {
                        let output = format!("Successfully updated federation config for {}", self.active_federation);
                        Ok(output)
                    },
                    Err(e) => Err(anyhow!("Failed to update federation config: {}", e)),
                }
            },
            FederationNetworkProposalType::EnableCrossFederation { target_federation } => {
                info!("Executing EnableCrossFederation proposal for target {}", target_federation);
                
                // Get current config
                let mut config = self.network_manager.get_federation_config(&self.active_federation).await?;
                
                // Update cross-federation settings
                config.allow_cross_federation = true;
                if !config.allowed_federations.contains(&target_federation) {
                    config.allowed_federations.push(target_federation.clone());
                }
                
                // Apply updated config
                match self.network_manager.update_federation_config(&self.active_federation, config).await {
                    Ok(_) => {
                        let output = format!("Successfully enabled cross-federation communication with {}", target_federation);
                        Ok(output)
                    },
                    Err(e) => Err(anyhow!("Failed to enable cross-federation communication: {}", e)),
                }
            },
            FederationNetworkProposalType::DisableCrossFederation { target_federation } => {
                info!("Executing DisableCrossFederation proposal for target {}", target_federation);
                
                // Get current config
                let mut config = self.network_manager.get_federation_config(&self.active_federation).await?;
                
                // Update cross-federation settings
                config.allowed_federations.retain(|f| f != &target_federation);
                if config.allowed_federations.is_empty() {
                    config.allow_cross_federation = false;
                }
                
                // Apply updated config
                match self.network_manager.update_federation_config(&self.active_federation, config).await {
                    Ok(_) => {
                        let output = format!("Successfully disabled cross-federation communication with {}", target_federation);
                        Ok(output)
                    },
                    Err(e) => Err(anyhow!("Failed to disable cross-federation communication: {}", e)),
                }
            },
            FederationNetworkProposalType::EnableWireGuard => {
                info!("Executing EnableWireGuard proposal for federation {}", self.active_federation);
                match self.network_manager.enable_federation_wireguard(&self.active_federation).await {
                    Ok(_) => {
                        let output = format!("Successfully enabled WireGuard for federation {}", self.active_federation);
                        Ok(output)
                    },
                    Err(e) => Err(anyhow!("Failed to enable WireGuard: {}", e)),
                }
            },
            FederationNetworkProposalType::DisableWireGuard => {
                info!("Executing DisableWireGuard proposal");
                
                // Get current config
                let mut config = self.network_manager.get_federation_config(&self.active_federation).await?;
                
                // Update WireGuard setting
                config.use_wireguard = false;
                
                // Apply updated config
                match self.network_manager.update_federation_config(&self.active_federation, config).await {
                    Ok(_) => {
                        let output = format!("Successfully disabled WireGuard for federation {}", self.active_federation);
                        Ok(output)
                    },
                    Err(e) => Err(anyhow!("Failed to disable WireGuard: {}", e)),
                }
            },
            FederationNetworkProposalType::AddBootstrapPeers { peers } => {
                info!("Executing AddBootstrapPeers proposal with {} peers", peers.len());
                
                // Get current config
                let mut config = self.network_manager.get_federation_config(&self.active_federation).await?;
                
                // Add bootstrap peers
                for peer in peers.iter() {
                    if !config.bootstrap_peers.contains(peer) {
                        config.bootstrap_peers.push(peer.clone());
                    }
                }
                
                // Apply updated config
                match self.network_manager.update_federation_config(&self.active_federation, config).await {
                    Ok(_) => {
                        let output = format!("Successfully added {} bootstrap peers", peers.len());
                        Ok(output)
                    },
                    Err(e) => Err(anyhow!("Failed to add bootstrap peers: {}", e)),
                }
            },
        };
        
        // Handle execution result
        match result {
            Ok(output) => {
                // Mark proposal as executed in governance service
                governance.update_proposal_status(proposal_id, ProposalStatus::Executed).await?;
                
                // Broadcast execution success
                let exec_message = FederationGovernanceMessage::ProposalExecution {
                    proposal_id: proposal_id.to_string(),
                    result: true,
                    output,
                };
                
                let message_json = serde_json::to_value(exec_message)
                    .map_err(|e| anyhow!("Failed to serialize execution message: {}", e))?;
                
                self.network_manager.broadcast_to_federation(
                    &self.active_federation,
                    "governance_execution",
                    message_json,
                ).await?;
                
                Ok(())
            },
            Err(e) => {
                warn!("Failed to execute network proposal {}: {}", proposal_id, e);
                
                // Broadcast execution failure
                let exec_message = FederationGovernanceMessage::ProposalExecution {
                    proposal_id: proposal_id.to_string(),
                    result: false,
                    output: format!("Execution failed: {}", e),
                };
                
                let message_json = serde_json::to_value(exec_message)
                    .map_err(|e| anyhow!("Failed to serialize execution message: {}", e))?;
                
                self.network_manager.broadcast_to_federation(
                    &self.active_federation,
                    "governance_execution",
                    message_json,
                ).await?;
                
                Err(e)
            },
        }
    }
    
    /// Sync governance data with federation peers
    pub async fn sync_with_federation(&self) -> Result<()> {
        // Get all local proposal IDs
        let local_proposals = {
            let governance = self.governance_service.read().await;
            governance.get_proposals().iter().map(|p| p.id.clone()).collect::<Vec<_>>()
        };
        
        // Create data sync request
        let sync_request = FederationGovernanceMessage::DataSyncRequest {
            proposal_ids: local_proposals,
        };
        
        // Convert to JSON
        let message_json = serde_json::to_value(sync_request)
            .map_err(|e| anyhow!("Failed to serialize sync request: {}", e))?;
        
        // Broadcast to current federation
        self.network_manager.broadcast_to_federation(
            &self.active_federation,
            "governance_sync_request",
            message_json,
        ).await?;
        
        Ok(())
    }
    
    /// Process incoming governance message
    pub async fn process_governance_message(&self, sender: &str, message_type: &str, content: serde_json::Value) -> Result<()> {
        match message_type {
            "governance_proposal" => {
                self.handle_proposal_message(sender, content).await?;
            },
            "governance_vote" => {
                self.handle_vote_message(sender, content).await?;
            },
            "governance_execution" => {
                self.handle_execution_message(sender, content).await?;
            },
            "governance_sync_request" => {
                self.handle_sync_request(sender, content).await?;
            },
            "governance_sync_response" => {
                self.handle_sync_response(sender, content).await?;
            },
            _ => {
                warn!("Unknown governance message type: {}", message_type);
            }
        }
        
        Ok(())
    }
    
    /// Handle proposal publication message
    async fn handle_proposal_message(&self, sender: &str, content: serde_json::Value) -> Result<()> {
        // Parse the message
        let message: FederationGovernanceMessage = serde_json::from_value(content)
            .map_err(|e| anyhow!("Failed to parse proposal message: {}", e))?;
        
        // Extract proposal
        if let FederationGovernanceMessage::ProposalPublication { proposal } = message {
            info!("Received proposal publication from {}: {}", sender, proposal.title);
            
            // Check if we already have this proposal
            let exists = {
                let governance = self.governance_service.read().await;
                governance.get_proposal(&proposal.id).is_some()
            };
            
            if !exists {
                // Add proposal to local governance
                let mut governance = self.governance_service.write().await;
                
                // TODO: Implement method to directly import a proposal
                // For now, we'll just create it with the same data
                
                info!("Imported proposal {} from peer {}", proposal.id, sender);
            }
        }
        
        Ok(())
    }
    
    /// Handle vote message
    async fn handle_vote_message(&self, sender: &str, content: serde_json::Value) -> Result<()> {
        // Parse the message
        let message: FederationGovernanceMessage = serde_json::from_value(content)
            .map_err(|e| anyhow!("Failed to parse vote message: {}", e))?;
        
        // Extract vote
        if let FederationGovernanceMessage::VoteCast { proposal_id, vote } = message {
            info!("Received vote from {} on proposal {}", sender, proposal_id);
            
            // Apply vote locally
            let mut governance = self.governance_service.write().await;
            
            // Check if proposal exists
            if governance.get_proposal(&proposal_id).is_none() {
                // We don't have this proposal, request sync
                self.request_proposal_sync(&[proposal_id]).await?;
                return Ok(());
            }
            
            // Cast the vote (this might need to be modified to directly apply the vote)
            governance.cast_vote(
                &proposal_id,
                &vote.member_id,
                vote.vote,
                vote.comment,
                vote.weight,
            ).await?;
            
            info!("Applied vote from {} on proposal {}", vote.member_id, proposal_id);
        }
        
        Ok(())
    }
    
    /// Handle execution message
    async fn handle_execution_message(&self, sender: &str, content: serde_json::Value) -> Result<()> {
        // Parse the message
        let message: FederationGovernanceMessage = serde_json::from_value(content)
            .map_err(|e| anyhow!("Failed to parse execution message: {}", e))?;
        
        // Extract execution result
        if let FederationGovernanceMessage::ProposalExecution { proposal_id, result, output } = message {
            info!("Received execution notification from {} for proposal {}: {}", 
                  sender, proposal_id, if result { "SUCCESS" } else { "FAILED" });
            
            // Update proposal status if successful
            if result {
                let mut governance = self.governance_service.write().await;
                
                // Check if proposal exists
                if let Some(proposal) = governance.get_proposal(&proposal_id) {
                    if proposal.status != ProposalStatus::Executed {
                        governance.update_proposal_status(&proposal_id, ProposalStatus::Executed).await?;
                        info!("Updated proposal {} status to Executed", proposal_id);
                    }
                } else {
                    // We don't have this proposal, request sync
                    self.request_proposal_sync(&[proposal_id]).await?;
                }
            }
            
            info!("Execution output: {}", output);
        }
        
        Ok(())
    }
    
    /// Handle sync request
    async fn handle_sync_request(&self, sender: &str, content: serde_json::Value) -> Result<()> {
        // Parse the message
        let message: FederationGovernanceMessage = serde_json::from_value(content)
            .map_err(|e| anyhow!("Failed to parse sync request: {}", e))?;
        
        // Extract proposal IDs
        if let FederationGovernanceMessage::DataSyncRequest { proposal_ids } = message {
            info!("Received governance sync request from {} for {} proposals", sender, proposal_ids.len());
            
            // Get all local proposals
            let governance = self.governance_service.read().await;
            let all_proposals = governance.get_proposals();
            
            // Find proposals we have that the requester doesn't
            let mut proposals_to_send = Vec::new();
            for proposal in all_proposals {
                if !proposal_ids.contains(&proposal.id) {
                    proposals_to_send.push(proposal.clone());
                }
            }
            
            if !proposals_to_send.is_empty() {
                // Create sync response
                let sync_response = FederationGovernanceMessage::DataSyncResponse {
                    proposals: proposals_to_send,
                };
                
                // Convert to JSON
                let message_json = serde_json::to_value(sync_response)
                    .map_err(|e| anyhow!("Failed to serialize sync response: {}", e))?;
                
                // Send to the requester directly
                // TODO: Implement direct message sending
                // For now, broadcast to the federation
                self.network_manager.broadcast_to_federation(
                    &self.active_federation,
                    "governance_sync_response",
                    message_json,
                ).await?;
                
                info!("Sent {} missing proposals to federation", proposals_to_send.len());
            }
        }
        
        Ok(())
    }
    
    /// Handle sync response
    async fn handle_sync_response(&self, sender: &str, content: serde_json::Value) -> Result<()> {
        // Parse the message
        let message: FederationGovernanceMessage = serde_json::from_value(content)
            .map_err(|e| anyhow!("Failed to parse sync response: {}", e))?;
        
        // Extract proposals
        if let FederationGovernanceMessage::DataSyncResponse { proposals } = message {
            info!("Received governance sync response from {} with {} proposals", sender, proposals.len());
            
            // Import missing proposals
            let mut governance = self.governance_service.write().await;
            
            for proposal in proposals {
                // Check if we already have this proposal
                if governance.get_proposal(&proposal.id).is_none() {
                    // TODO: Implement direct proposal import
                    // For now, we'll just log
                    info!("Would import proposal {}: {}", proposal.id, proposal.title);
                }
            }
        }
        
        Ok(())
    }
    
    /// Request sync for specific proposals
    async fn request_proposal_sync(&self, proposal_ids: &[String]) -> Result<()> {
        // Create sync request for specific proposals
        let sync_request = FederationGovernanceMessage::DataSyncRequest {
            proposal_ids: proposal_ids.to_vec(),
        };
        
        // Convert to JSON
        let message_json = serde_json::to_value(sync_request)
            .map_err(|e| anyhow!("Failed to serialize sync request: {}", e))?;
        
        // Broadcast to current federation
        self.network_manager.broadcast_to_federation(
            &self.active_federation,
            "governance_sync_request",
            message_json,
        ).await?;
        
        Ok(())
    }
    
    /// Switch to a different federation
    pub async fn switch_federation(&mut self, federation_id: &str) -> Result<()> {
        // Verify federation exists
        let federations = self.network_manager.get_federations().await;
        if !federations.contains(&federation_id.to_string()) {
            return Err(anyhow!("Federation {} does not exist", federation_id));
        }
        
        // Update active federation
        self.active_federation = federation_id.to_string();
        
        // Switch federation in network manager
        self.network_manager.set_active_federation(federation_id).await?;
        
        Ok(())
    }
} 