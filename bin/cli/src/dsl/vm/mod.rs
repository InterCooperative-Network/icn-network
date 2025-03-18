/// Virtual Machine Module
///
/// This module implements a simple virtual machine for executing the DSL AST.
/// It provides a secure and deterministic execution environment for governance rules,
/// economic transactions, and resource allocation logic.

use crate::dsl::parser::{Ast, AstNode, ProposalNode, AssetNode, ExecutionStepNode, VotingMethodNode};
use crate::dsl::DslEvent;
use anyhow::{Context, Result, anyhow};
use tokio::sync::mpsc;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Virtual Machine for executing DSL code
pub struct VirtualMachine {
    /// Event sender for emitting events
    event_sender: mpsc::Sender<DslEvent>,
    /// VM state
    state: Arc<Mutex<VmState>>,
}

/// VM State
#[derive(Debug)]
struct VmState {
    /// Registered proposals
    proposals: HashMap<String, ProposalNode>,
    /// Registered assets
    assets: HashMap<String, AssetNode>,
    /// Asset balances
    balances: HashMap<String, HashMap<String, u64>>,
    /// Votes on proposals
    votes: HashMap<String, HashMap<String, VoteRecord>>,
}

/// Vote record
#[derive(Debug, Clone)]
struct VoteRecord {
    /// Voter ID
    voter_id: String,
    /// Vote type
    vote_type: crate::dsl::VoteType,
    /// Vote weight
    weight: f64,
}

impl VmState {
    /// Create a new VM state
    fn new() -> Self {
        Self {
            proposals: HashMap::new(),
            assets: HashMap::new(),
            balances: HashMap::new(),
            votes: HashMap::new(),
        }
    }
}

impl VirtualMachine {
    /// Create a new VM
    pub fn new(event_sender: mpsc::Sender<DslEvent>) -> Self {
        Self {
            event_sender,
            state: Arc::new(Mutex::new(VmState::new())),
        }
    }

    /// Execute an AST
    pub async fn execute(&mut self, ast: Ast) -> Result<()> {
        for node in ast.nodes {
            self.execute_node(node).await?;
        }
        Ok(())
    }

    /// Execute a single AST node
    async fn execute_node(&self, node: AstNode) -> Result<()> {
        match node {
            AstNode::Proposal(proposal) => self.register_proposal(proposal).await?,
            AstNode::Asset(asset) => self.register_asset(asset).await?,
            AstNode::ExecutionStep(step) => self.execute_step(step).await?,
            _ => {
                // Log unhandled node type
                self.emit_event(DslEvent::Log(format!("Unhandled AST node type"))).await?;
            }
        }
        Ok(())
    }

    /// Register a proposal
    async fn register_proposal(&self, proposal: ProposalNode) -> Result<()> {
        let mut state = self.state.lock().unwrap();
        state.proposals.insert(proposal.id.clone(), proposal.clone());
        
        // Emit event
        drop(state); // Release lock before async operation
        self.emit_event(DslEvent::ProposalCreated {
            id: proposal.id,
            title: proposal.title,
            description: proposal.description,
        }).await?;
        
        Ok(())
    }

    /// Register an asset
    async fn register_asset(&self, asset: AssetNode) -> Result<()> {
        let mut state = self.state.lock().unwrap();
        state.assets.insert(asset.id.clone(), asset);
        
        // Emit event
        drop(state); // Release lock before async operation
        self.emit_event(DslEvent::Log(format!("Asset registered"))).await?;
        
        Ok(())
    }

    /// Execute a step
    async fn execute_step(&self, step: ExecutionStepNode) -> Result<()> {
        match step.action.as_str() {
            "allocate_funds" => {
                // Implement fund allocation logic
                self.emit_event(DslEvent::Log(format!("Funds allocated"))).await?;
            }
            "transfer" => {
                // Get parameters
                let from = step.params.get("from").ok_or_else(|| anyhow!("Missing 'from' parameter"))?;
                let to = step.params.get("to").ok_or_else(|| anyhow!("Missing 'to' parameter"))?;
                let amount_str = step.params.get("amount").ok_or_else(|| anyhow!("Missing 'amount' parameter"))?;
                let asset_type = step.params.get("asset_type").ok_or_else(|| anyhow!("Missing 'asset_type' parameter"))?;
                
                let amount = amount_str.parse::<u64>().context("Invalid amount")?;
                
                // Perform transfer
                let mut state = self.state.lock().unwrap();
                
                // Check if from account has sufficient balance
                let from_balances = state.balances.entry(from.clone()).or_insert_with(HashMap::new);
                let from_balance = from_balances.entry(asset_type.clone()).or_insert(0);
                
                if *from_balance < amount {
                    return Err(anyhow!("Insufficient balance"));
                }
                
                // Update balances
                *from_balance -= amount;
                
                let to_balances = state.balances.entry(to.clone()).or_insert_with(HashMap::new);
                let to_balance = to_balances.entry(asset_type.clone()).or_insert(0);
                *to_balance += amount;
                
                // Emit event
                drop(state); // Release lock before async operation
                self.emit_event(DslEvent::Transaction {
                    from: from.clone(),
                    to: to.clone(),
                    amount,
                    asset_type: asset_type.clone(),
                }).await?;
            }
            "log" => {
                let message = step.params.get("message").unwrap_or(&"".to_string()).clone();
                self.emit_event(DslEvent::Log(message)).await?;
            }
            _ => {
                self.emit_event(DslEvent::Error(format!("Unknown action: {}", step.action))).await?;
                return Err(anyhow!("Unknown action: {}", step.action));
            }
        }
        
        Ok(())
    }

    /// Cast a vote on a proposal
    pub async fn cast_vote(
        &self, 
        proposal_id: &str, 
        voter_id: &str, 
        vote: crate::dsl::VoteType, 
        weight: f64
    ) -> Result<()> {
        let mut state = self.state.lock().unwrap();
        
        // Check if proposal exists
        if !state.proposals.contains_key(proposal_id) {
            return Err(anyhow!("Proposal not found: {}", proposal_id));
        }
        
        // Register vote
        let proposal_votes = state.votes.entry(proposal_id.to_string()).or_insert_with(HashMap::new);
        proposal_votes.insert(voter_id.to_string(), VoteRecord {
            voter_id: voter_id.to_string(),
            vote_type: vote.clone(),
            weight,
        });
        
        // Emit event
        drop(state); // Release lock before async operation
        self.emit_event(DslEvent::VoteCast {
            proposal_id: proposal_id.to_string(),
            voter_id: voter_id.to_string(),
            vote,
        }).await?;
        
        Ok(())
    }

    /// Execute a proposal based on votes
    pub async fn execute_proposal(&self, proposal_id: &str) -> Result<()> {
        let state = self.state.lock().unwrap();
        
        // Check if proposal exists
        let proposal = state.proposals.get(proposal_id).ok_or_else(|| anyhow!("Proposal not found: {}", proposal_id))?;
        
        // Get votes
        let votes = state.votes.get(proposal_id).cloned().unwrap_or_default();
        
        // Calculate result based on voting method
        let approved = match proposal.voting_method {
            VotingMethodNode::Majority => {
                // Simple majority calculation
                let mut yes_votes = 0.0;
                let mut total_votes = 0.0;
                
                for vote in votes.values() {
                    match vote.vote_type {
                        crate::dsl::VoteType::Yes => {
                            yes_votes += vote.weight;
                            total_votes += vote.weight;
                        }
                        crate::dsl::VoteType::No => {
                            total_votes += vote.weight;
                        }
                        crate::dsl::VoteType::Abstain => {
                            // Abstain votes don't count
                        }
                        _ => {
                            // Ranked choice or other vote types not handled in majority voting
                        }
                    }
                }
                
                // Check if there are any votes
                if total_votes > 0.0 {
                    yes_votes / total_votes > 0.5
                } else {
                    false
                }
            }
            VotingMethodNode::RankedChoice => {
                // Placeholder for ranked choice
                // In a real implementation, you would implement the ranked choice algorithm
                false
            }
            VotingMethodNode::Quadratic => {
                // Placeholder for quadratic voting
                // In a real implementation, you would implement the quadratic voting algorithm
                false
            }
            VotingMethodNode::Custom { threshold, .. } => {
                // Custom voting with threshold
                let mut yes_votes = 0.0;
                let mut total_votes = 0.0;
                
                for vote in votes.values() {
                    match vote.vote_type {
                        crate::dsl::VoteType::Yes => {
                            yes_votes += vote.weight;
                            total_votes += vote.weight;
                        }
                        crate::dsl::VoteType::No => {
                            total_votes += vote.weight;
                        }
                        crate::dsl::VoteType::Abstain => {
                            // Abstain votes don't count
                        }
                        _ => {
                            // Ranked choice or other vote types not handled in custom voting
                        }
                    }
                }
                
                // Check if there are any votes and if yes votes exceed threshold
                if total_votes > 0.0 {
                    yes_votes / total_votes > threshold
                } else {
                    false
                }
            }
        };
        
        // Release lock before async operations
        drop(state);
        
        // Emit event with result
        self.emit_event(DslEvent::ProposalExecuted {
            id: proposal_id.to_string(),
            result: approved,
        }).await?;
        
        if approved {
            // Execute proposal actions if approved
            let state = self.state.lock().unwrap();
            let proposal = state.proposals.get(proposal_id).unwrap();
            
            // Release lock before executing steps
            let steps = proposal.execution.clone();
            drop(state);
            
            for step in steps {
                self.execute_step(step).await?;
            }
        }
        
        Ok(())
    }

    /// Emit an event
    async fn emit_event(&self, event: DslEvent) -> Result<()> {
        self.event_sender.send(event).await.context("Failed to send event")?;
        Ok(())
    }
}
