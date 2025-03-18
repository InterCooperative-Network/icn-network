/// Domain-Specific Language (DSL) for ICN Network
///
/// This module implements a Domain-Specific Language (DSL) and Virtual Machine (VM) 
/// for expressing cooperative governance rules, economic transactions, 
/// and resource allocations in a secure and deterministic way.
///
/// The DSL allows for expressing governance rules, proposals, voting methods,
/// and economic transactions using a clear and concise syntax, while the VM
/// provides a secure execution environment for these rules.

pub mod vm;
pub mod parser;
pub mod stdlib;
pub mod integration;

use anyhow::Result;
use tokio::sync::mpsc;
use std::path::Path;

/// Main entry point for the DSL system
pub struct DslSystem {
    /// Channel for sending events from the VM to other system components
    event_sender: mpsc::Sender<DslEvent>,
}

/// Events that can be emitted by the DSL VM during execution
#[derive(Debug, Clone)]
pub enum DslEvent {
    /// A proposal was created
    ProposalCreated {
        id: String,
        title: String,
        description: String,
    },
    /// A vote was cast on a proposal
    VoteCast {
        proposal_id: String,
        voter_id: String,
        vote: VoteType,
    },
    /// A proposal was executed
    ProposalExecuted {
        id: String,
        result: bool,
    },
    /// An economic transaction occurred
    Transaction {
        from: String,
        to: String,
        amount: u64,
        asset_type: String,
    },
    /// A log message was emitted
    Log(String),
    /// An error occurred during execution
    Error(String),
}

/// Type of vote that can be cast
#[derive(Debug, Clone)]
pub enum VoteType {
    Yes,
    No,
    Abstain,
    RankedChoice(Vec<String>),
}

impl DslSystem {
    /// Create a new DSL system
    pub fn new(event_sender: mpsc::Sender<DslEvent>) -> Self {
        Self { event_sender }
    }

    /// Execute a DSL script from a string
    pub async fn execute_script(&self, script: &str) -> Result<()> {
        let ast = parser::parse_script(script)?;
        let mut vm = vm::VirtualMachine::new(self.event_sender.clone());
        vm.execute(ast).await
    }

    /// Execute a DSL script from a file
    pub async fn execute_script_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let script = std::fs::read_to_string(path)?;
        self.execute_script(&script).await
    }
}

/// Create a default DSL system with an unbounded event channel
pub async fn create_default_system() -> (DslSystem, mpsc::Receiver<DslEvent>) {
    let (tx, rx) = mpsc::channel(100);
    let system = DslSystem::new(tx);
    (system, rx)
}
