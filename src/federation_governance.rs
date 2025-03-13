use std::error::Error;
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use crate::identity::Identity;
use crate::storage::Storage;

// Governance error types
#[derive(Debug)]
pub enum GovernanceError {
    InvalidProposal(String),
    InvalidVote(String),
    ProposalNotFound(String),
    VotingPeriodExpired(String),
    InsufficientVotes(String),
    InvalidQuorum(String),
    DisputeNotFound(String),
    InvalidResolution(String),
}

impl fmt::Display for GovernanceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GovernanceError::InvalidProposal(msg) => write!(f, "Invalid proposal: {}", msg),
            GovernanceError::InvalidVote(msg) => write!(f, "Invalid vote: {}", msg),
            GovernanceError::ProposalNotFound(msg) => write!(f, "Proposal not found: {}", msg),
            GovernanceError::VotingPeriodExpired(msg) => write!(f, "Voting period expired: {}", msg),
            GovernanceError::InsufficientVotes(msg) => write!(f, "Insufficient votes: {}", msg),
            GovernanceError::InvalidQuorum(msg) => write!(f, "Invalid quorum: {}", msg),
            GovernanceError::DisputeNotFound(msg) => write!(f, "Dispute not found: {}", msg),
            GovernanceError::InvalidResolution(msg) => write!(f, "Invalid resolution: {}", msg),
        }
    }
}

impl Error for GovernanceError {}

// Proposal types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProposalType {
    PolicyChange,
    MemberAddition,
    MemberRemoval,
    CreditLimitAdjustment,
    FeeAdjustment,
    DisputeResolution,
}

// Proposal status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProposalStatus {
    Draft,
    Active,
    Passed,
    Failed,
    Expired,
    Executed,
}

// Proposal structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    pub id: String,
    pub federation_id: String,
    pub proposal_type: ProposalType,
    pub title: String,
    pub description: String,
    pub created_by: String,
    pub created_at: u64,
    pub voting_start: u64,
    pub voting_end: u64,
    pub quorum: u64,
    pub status: ProposalStatus,
    pub votes: Vec<Vote>,
    pub changes: serde_json::Value,
}

// Vote structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vote {
    pub member_did: String,
    pub cooperative_id: String,
    pub vote: bool,
    pub timestamp: u64,
    pub signature: Vec<u8>,
}

// Dispute structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dispute {
    pub id: String,
    pub federation_id: String,
    pub transaction_id: String,
    pub complainant_did: String,
    pub respondent_did: String,
    pub description: String,
    pub created_at: u64,
    pub status: DisputeStatus,
    pub evidence: Vec<Evidence>,
    pub resolution: Option<Resolution>,
}

// Dispute status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DisputeStatus {
    Open,
    UnderReview,
    Resolved,
    Dismissed,
    Escalated,
}

// Evidence structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    pub id: String,
    pub submitted_by: String,
    pub timestamp: u64,
    pub description: String,
    pub data: Vec<u8>,
    pub signature: Vec<u8>,
}

// Resolution structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resolution {
    pub id: String,
    pub resolved_by: String,
    pub timestamp: u64,
    pub decision: String,
    pub actions: Vec<String>,
    pub signature: Vec<u8>,
}

// Federation governance system
pub struct FederationGovernance {
    identity: Identity,
    storage: Storage,
}

impl FederationGovernance {
    // Create a new governance system
    pub fn new(identity: Identity, storage: Storage) -> Self {
        FederationGovernance {
            identity,
            storage,
        }
    }

    // Create a new proposal
    pub fn create_proposal(
        &self,
        federation_id: &str,
        proposal_type: ProposalType,
        title: &str,
        description: &str,
        voting_duration: u64,
        quorum: u64,
        changes: serde_json::Value,
    ) -> Result<Proposal, Box<dyn Error>> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();

        let proposal = Proposal {
            id: format!("prop-{}", now),
            federation_id: federation_id.to_string(),
            proposal_type,
            title: title.to_string(),
            description: description.to_string(),
            created_by: self.identity.did.clone(),
            created_at: now,
            voting_start: now,
            voting_end: now + voting_duration,
            quorum,
            status: ProposalStatus::Active,
            votes: Vec::new(),
            changes,
        };

        // Store the proposal
        self.storage.put_json(
            &format!("proposals/{}", proposal.id),
            &proposal,
        )?;

        Ok(proposal)
    }

    // Vote on a proposal
    pub fn vote(
        &self,
        proposal_id: &str,
        vote: bool,
    ) -> Result<(), Box<dyn Error>> {
        let mut proposal: Proposal = self.storage.get_json(
            &format!("proposals/{}", proposal_id),
        )?;

        // Check if voting period is still active
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();
        if now > proposal.voting_end {
            return Err(Box::new(GovernanceError::VotingPeriodExpired(
                "Voting period has ended".to_string(),
            )));
        }

        // Check if member has already voted
        if proposal.votes.iter().any(|v| v.member_did == self.identity.did) {
            return Err(Box::new(GovernanceError::InvalidVote(
                "Member has already voted".to_string(),
            )));
        }

        // Create and sign the vote
        let vote_data = serde_json::to_vec(&(proposal_id, vote, now))?;
        let signature = self.identity.sign(&vote_data)?;

        let vote = Vote {
            member_did: self.identity.did.clone(),
            cooperative_id: self.identity.coop_id.clone(),
            vote,
            timestamp: now,
            signature: signature.to_bytes().to_vec(),
        };

        proposal.votes.push(vote);
        self.storage.put_json(
            &format!("proposals/{}", proposal_id),
            &proposal,
        )?;

        Ok(())
    }

    // Process a proposal
    pub fn process_proposal(
        &self,
        proposal_id: &str,
    ) -> Result<(), Box<dyn Error>> {
        let mut proposal: Proposal = self.storage.get_json(
            &format!("proposals/{}", proposal_id),
        )?;

        // Check if voting period has ended
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();
        if now <= proposal.voting_end {
            return Err(Box::new(GovernanceError::VotingPeriodExpired(
                "Voting period has not ended".to_string(),
            )));
        }

        // Check if quorum was reached
        if proposal.votes.len() < proposal.quorum as usize {
            proposal.status = ProposalStatus::Failed;
            self.storage.put_json(
                &format!("proposals/{}", proposal_id),
                &proposal,
            )?;
            return Err(Box::new(GovernanceError::InsufficientVotes(
                "Quorum not reached".to_string(),
            )));
        }

        // Calculate vote results
        let yes_votes = proposal.votes.iter().filter(|v| v.vote).count();
        let total_votes = proposal.votes.len();
        let passed = yes_votes > total_votes / 2;

        // Update proposal status
        proposal.status = if passed {
            ProposalStatus::Passed
        } else {
            ProposalStatus::Failed
        };

        // Store updated proposal
        self.storage.put_json(
            &format!("proposals/{}", proposal_id),
            &proposal,
        )?;

        // If passed, apply the changes
        if passed {
            self.apply_proposal_changes(&proposal)?;
        }

        Ok(())
    }

    // Create a dispute
    pub fn create_dispute(
        &self,
        federation_id: &str,
        transaction_id: &str,
        respondent_did: &str,
        description: &str,
        evidence: Vec<Evidence>,
    ) -> Result<Dispute, Box<dyn Error>> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();

        let dispute = Dispute {
            id: format!("disp-{}", now),
            federation_id: federation_id.to_string(),
            transaction_id: transaction_id.to_string(),
            complainant_did: self.identity.did.clone(),
            respondent_did: respondent_did.to_string(),
            description: description.to_string(),
            created_at: now,
            status: DisputeStatus::Open,
            evidence,
            resolution: None,
        };

        // Store the dispute
        self.storage.put_json(
            &format!("disputes/{}", dispute.id),
            &dispute,
        )?;

        Ok(dispute)
    }

    // Add evidence to a dispute
    pub fn add_evidence(
        &self,
        dispute_id: &str,
        description: &str,
        data: Vec<u8>,
    ) -> Result<(), Box<dyn Error>> {
        let mut dispute: Dispute = self.storage.get_json(
            &format!("disputes/{}", dispute_id),
        )?;

        // Verify the member is involved in the dispute
        if dispute.complainant_did != self.identity.did && 
           dispute.respondent_did != self.identity.did {
            return Err(Box::new(GovernanceError::InvalidResolution(
                "Member is not involved in the dispute".to_string(),
            )));
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();

        // Create and sign the evidence
        let evidence_data = serde_json::to_vec(&(dispute_id, description, &data, now))?;
        let signature = self.identity.sign(&evidence_data)?;

        let evidence = Evidence {
            id: format!("evid-{}", now),
            submitted_by: self.identity.did.clone(),
            timestamp: now,
            description: description.to_string(),
            data,
            signature: signature.to_bytes().to_vec(),
        };

        dispute.evidence.push(evidence);
        dispute.status = DisputeStatus::UnderReview;
        
        self.storage.put_json(
            &format!("disputes/{}", dispute_id),
            &dispute,
        )?;

        Ok(())
    }

    // Resolve a dispute
    pub fn resolve_dispute(
        &self,
        dispute_id: &str,
        decision: &str,
        actions: Vec<String>,
    ) -> Result<(), Box<dyn Error>> {
        let mut dispute: Dispute = self.storage.get_json(
            &format!("disputes/{}", dispute_id),
        )?;

        // Verify the member has authority to resolve disputes
        // In a real implementation, this would check for specific roles/permissions
        if !self.has_dispute_resolution_authority() {
            return Err(Box::new(GovernanceError::InvalidResolution(
                "Member lacks authority to resolve disputes".to_string(),
            )));
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();

        // Create and sign the resolution
        let resolution_data = serde_json::to_vec(&(dispute_id, decision, &actions, now))?;
        let signature = self.identity.sign(&resolution_data)?;

        let resolution = Resolution {
            id: format!("res-{}", now),
            resolved_by: self.identity.did.clone(),
            timestamp: now,
            decision: decision.to_string(),
            actions,
            signature: signature.to_bytes().to_vec(),
        };

        dispute.resolution = Some(resolution);
        dispute.status = DisputeStatus::Resolved;
        
        self.storage.put_json(
            &format!("disputes/{}", dispute_id),
            &dispute,
        )?;

        Ok(())
    }

    // Helper function to check dispute resolution authority
    fn has_dispute_resolution_authority(&self) -> bool {
        // In a real implementation, this would check for specific roles/permissions
        // For now, we'll assume the creator of the federation has this authority
        true
    }

    // Helper function to apply proposal changes
    fn apply_proposal_changes(&self, proposal: &Proposal) -> Result<(), Box<dyn Error>> {
        match proposal.proposal_type {
            ProposalType::PolicyChange => {
                // Apply policy changes to the federation
                let mut federation: crate::federation::Federation = self.storage.get_json(
                    &format!("federations/{}", proposal.federation_id),
                )?;
                // Apply changes to federation policies
                // This would need to be implemented based on the specific changes
                self.storage.put_json(
                    &format!("federations/{}", proposal.federation_id),
                    &federation,
                )?;
            },
            ProposalType::MemberAddition => {
                // Add new member to the federation
                let mut federation: crate::federation::Federation = self.storage.get_json(
                    &format!("federations/{}", proposal.federation_id),
                )?;
                // Add new member based on proposal changes
                // This would need to be implemented based on the specific changes
                self.storage.put_json(
                    &format!("federations/{}", proposal.federation_id),
                    &federation,
                )?;
            },
            ProposalType::MemberRemoval => {
                // Remove member from the federation
                let mut federation: crate::federation::Federation = self.storage.get_json(
                    &format!("federations/{}", proposal.federation_id),
                )?;
                // Remove member based on proposal changes
                // This would need to be implemented based on the specific changes
                self.storage.put_json(
                    &format!("federations/{}", proposal.federation_id),
                    &federation,
                )?;
            },
            ProposalType::CreditLimitAdjustment => {
                // Adjust credit limits for members
                // This would need to be implemented based on the specific changes
            },
            ProposalType::FeeAdjustment => {
                // Adjust transaction fees
                let mut federation: crate::federation::Federation = self.storage.get_json(
                    &format!("federations/{}", proposal.federation_id),
                )?;
                // Apply fee changes based on proposal changes
                // This would need to be implemented based on the specific changes
                self.storage.put_json(
                    &format!("federations/{}", proposal.federation_id),
                    &federation,
                )?;
            },
            ProposalType::DisputeResolution => {
                // Apply dispute resolution actions
                // This would need to be implemented based on the specific changes
            },
        }

        Ok(())
    }
} 