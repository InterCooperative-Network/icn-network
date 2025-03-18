use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identity {
    pub id: String,
    pub roles: HashSet<Role>,
    pub reputation: f64,
    pub joined_at: u64,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum Role {
    Admin,
    Member,
    Facilitator,
    Observer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    pub id: String,
    pub title: String,
    pub description: String,
    pub proposer: String, // Identity ID
    pub voting_method: VotingMethod,
    pub execution: Vec<Action>,
    pub rejection: Vec<Action>,
    pub created_at: u64,
    pub expires_at: Option<u64>,
    pub state: ProposalState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProposalState {
    Draft,
    Active,
    Passed,
    Rejected,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VotingMethod {
    pub method_type: VotingType,
    pub quorum: f64,          // 0.0 to 1.0
    pub threshold: Threshold,
    pub options: Vec<String>, // For ranked choice or multiple choice
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VotingType {
    Simple,           // Yes/No
    RankedChoice,    // Rank options in order
    Quadratic,       // Quadratic voting with credits
    Consensus,       // Full consensus required
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Threshold {
    Majority,
    SuperMajority(f64),  // e.g. 0.66 for 66%
    Unanimous,
    Custom(f64),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Action {
    AllocateFunds {
        asset: String,
        amount: u64,
        recipient: String,
    },
    UpdateRole {
        identity: String,
        role: Role,
        add: bool,  // true = add role, false = remove role
    },
    UpdatePolicy {
        key: String,
        value: String,
    },
    Custom {
        action_type: String,
        params: HashMap<String, String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vote {
    pub proposal_id: String,
    pub voter: String,
    pub choice: VoteChoice,
    pub weight: f64,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VoteChoice {
    Yes,
    No,
    Abstain,
    Ranked(Vec<usize>),      // Indices into proposal.options
    Quadratic(Vec<i64>),     // Positive or negative vote weights
}

// Trait for executing actions
#[async_trait::async_trait]
pub trait ActionExecutor {
    async fn execute(&self, action: &Action) -> Result<(), Box<dyn std::error::Error>>;
}

// Trait for voting method implementations
#[async_trait::async_trait]
pub trait VotingStrategy {
    async fn tally_votes(&self, votes: &[Vote], method: &VotingMethod) -> Result<ProposalState, Box<dyn std::error::Error>>;
} 