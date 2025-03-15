use std::error::Error;
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use serde_json::{self, json, Value};
use crate::identity::Identity;
use crate::storage::Storage;
use crate::reputation::{ReputationSystem, AttestationType, Evidence};

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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

// Evidence for governance disputes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceEvidence {
    pub id: String,
    pub submitted_by: String,
    pub signature: Vec<u8>,
}

// Dispute resolution for governance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dispute {
    pub id: String,
    pub proposal_id: String,
    pub raised_by: String,
    pub reason: String,
    pub evidence: Vec<GovernanceEvidence>,
    pub resolution: Option<DisputeResolution>,
    pub created_at: u64,
}

// Resolution for a dispute
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisputeResolution {
    pub resolved_by: String,
    pub decision: String,
    pub evidence: Vec<GovernanceEvidence>,
    pub timestamp: u64,
    pub signature: Vec<u8>,
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
    identity: Arc<Identity>,
    storage: Arc<Storage>,
    reputation: Option<Arc<ReputationSystem>>,
}

impl FederationGovernance {
    // Create a new governance system
    pub fn new(identity: Arc<Identity>, storage: Arc<Storage>) -> Self {
        FederationGovernance {
            identity,
            storage,
            reputation: None,
        }
    }
    
    // Set the reputation system (called after initialization)
    pub fn set_reputation_system(&mut self, reputation: Arc<ReputationSystem>) {
        self.reputation = Some(reputation);
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

        // Clone the proposal_type for later use
        let proposal_type_clone = proposal_type.clone();

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
        println!("Storing proposal at: proposals/{}", proposal.id);
        self.storage.put_json(
            &format!("proposals/{}", proposal.id),
            &proposal,
        )?;
        
        // Add reputation for creating a quality proposal
        if let Some(reputation) = &self.reputation {
            // Creating a proposal gives a small reputation boost
            let evidence = vec![
                Evidence {
                    evidence_type: "proposal_created".to_string(),
                    evidence_id: proposal.id.clone(),
                    description: format!("Created proposal: {}", title),
                    timestamp: now,
                    data: Some(serde_json::to_value(&proposal)?),
                }
            ];
            
            // Try to create an attestation for governance participation
            println!("Creating attestation for governance participation");
            let _ = reputation.attestation_manager().create_attestation(
                &self.identity.did,
                AttestationType::GovernanceQuality,
                0.5, // Moderate score for creating a proposal
                serde_json::json!({
                    "action": "proposal_creation",
                    "proposal_type": format!("{:?}", proposal_type_clone),
                }),
                evidence,
                1, // Self-attestation
                Some(180), // Valid for 180 days
            );
        }

        Ok(proposal)
    }

    // Vote on a proposal
    pub async fn vote(
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

        let vote_entry = Vote {
            member_did: self.identity.did.clone(),
            cooperative_id: self.identity.coop_id.clone(),
            vote,
            timestamp: now,
            signature: signature.to_bytes().to_vec(),
        };

        proposal.votes.push(vote_entry);
        self.storage.put_json(
            &format!("proposals/{}", proposal_id),
            &proposal,
        )?;
        
        // Add reputation for voting participation
        if let Some(reputation) = &self.reputation {
            // Voting gives a small reputation boost
            let evidence = vec![
                Evidence {
                    evidence_type: "vote_cast".to_string(),
                    evidence_id: format!("{}:{}", proposal_id, now),
                    description: format!("Voted on proposal: {}", proposal.title),
                    timestamp: now,
                    data: Some(serde_json::json!({
                        "proposal_id": proposal_id,
                        "vote": vote,
                    })),
                }
            ];
            
            // Try to create an attestation for governance participation
            let _ = reputation.attestation_manager().create_attestation(
                &self.identity.did,
                AttestationType::GovernanceQuality,
                0.3, // Small score for basic voting
                serde_json::json!({
                    "action": "vote_participation",
                    "proposal_type": format!("{:?}", proposal.proposal_type),
                }),
                evidence,
                1, // Self-attestation
                Some(90), // Valid for 90 days
            );
        }

        Ok(())
    }

    // Process a proposal and update participant reputation
    pub async fn process_proposal(
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
        
        // Update reputation for all participants
        if let Some(reputation) = &self.reputation {
            // For the proposal creator, add reputation based on outcome
            let proposal_quality_score = if passed {
                // Higher score if the proposal was successful
                0.7 
            } else {
                // Lower score if the proposal failed
                0.2
            };
            
            let creator_evidence = vec![
                Evidence {
                    evidence_type: "proposal_outcome".to_string(),
                    evidence_id: format!("{}:outcome", proposal_id),
                    description: format!("Proposal outcome: {}", 
                        if passed { "Passed" } else { "Failed" }),
                    timestamp: now,
                    data: Some(serde_json::json!({
                        "proposal_id": proposal_id,
                        "passed": passed,
                        "yes_votes": yes_votes,
                        "total_votes": total_votes,
                    })),
                }
            ];
            
            // Proposal creator reputation update
            let _ = reputation.attestation_manager().create_attestation(
                &proposal.created_by,
                AttestationType::GovernanceQuality,
                proposal_quality_score,
                serde_json::json!({
                    "action": "proposal_outcome",
                    "proposal_type": format!("{:?}", proposal.proposal_type),
                    "passed": passed,
                }),
                creator_evidence,
                1, 
                Some(180), // Valid for 180 days
            );
            
            // For each voter, minor reputation boost for participation
            for vote in &proposal.votes {
                // Check if voter was on the winning side (predicted correctly)
                let vote_aligned_with_outcome = vote.vote == passed;
                
                // Higher score for votes that aligned with final outcome
                let voter_score = if vote_aligned_with_outcome {
                    0.4 // Better score for "correct" votes
                } else {
                    0.2 // Lower score for "incorrect" votes but still positive for participation
                };
                
                let vote_evidence = vec![
                    Evidence {
                        evidence_type: "vote_outcome".to_string(),
                        evidence_id: format!("{}:vote_outcome:{}", proposal_id, vote.member_did),
                        description: format!("Vote alignment with outcome: {}", 
                            if vote_aligned_with_outcome { "Aligned" } else { "Not aligned" }),
                        timestamp: now,
                        data: Some(serde_json::json!({
                            "proposal_id": proposal_id,
                            "vote": vote.vote,
                            "outcome": passed,
                            "aligned": vote_aligned_with_outcome,
                        })),
                    }
                ];
                
                // Create attestation for voter
                let _ = reputation.attestation_manager().create_attestation(
                    &vote.member_did,
                    AttestationType::GovernanceQuality,
                    voter_score,
                    serde_json::json!({
                        "action": "vote_outcome",
                        "proposal_type": format!("{:?}", proposal.proposal_type),
                        "vote_aligned": vote_aligned_with_outcome,
                    }),
                    vote_evidence,
                    1,
                    Some(120), // Valid for 120 days
                );
            }
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
            proposal_id: format!("{}:{}", federation_id, transaction_id),
            raised_by: self.identity.did.clone(),
            reason: description.to_string(),
            evidence,
            resolution: None,
            created_at: now,
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
        if dispute.raised_by != self.identity.did {
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

        let evidence = GovernanceEvidence {
            id: format!("evid-{}", now),
            submitted_by: self.identity.did.clone(),
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

        let resolution = DisputeResolution {
            resolved_by: self.identity.did.clone(),
            decision: decision.to_string(),
            evidence: Vec::new(),
            timestamp: now,
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

    // Add a comment or deliberation to a proposal
    pub async fn add_deliberation(
        &self,
        proposal_id: &str,
        comment: &str,
        references: Vec<String>, // References to other comments or evidence
    ) -> Result<Deliberation, Box<dyn Error>> {
        // Check if the proposal exists
        let proposal_key = format!("proposals/{}", proposal_id);
        println!("Checking if proposal exists at: {}", proposal_key);
        let proposal: Proposal = self.storage.get_json(
            &proposal_key,
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
        
        // Create and sign the deliberation
        let deliberation_data = format!("{}:{}:{}", proposal_id, comment, now);
        let signature = self.identity.sign(deliberation_data.as_bytes())?;
        
        let deliberation = Deliberation {
            id: format!("delib-{}:{}", proposal_id, now),
            proposal_id: proposal_id.to_string(),
            member_did: self.identity.did.clone(),
            comment: comment.to_string(),
            timestamp: now,
            references: references.clone(),
            signature: signature.to_bytes().to_vec(),
        };
        
        // Store the deliberation
        let deliberation_key = format!("deliberations/{}/{}", proposal_id, deliberation.id);
        println!("Storing deliberation at: {}", deliberation_key);
        match self.storage.put_json(&deliberation_key, &deliberation) {
            Ok(_) => println!("Successfully stored deliberation"),
            Err(e) => {
                println!("Error storing deliberation: {:?}", e);
                return Err(e);
            }
        }
        
        // Add to list of deliberations for this proposal
        let deliberations_key = format!("proposal_deliberations/{}", proposal_id);
        println!("Updating deliberation list at: {}", deliberations_key);
        let mut deliberation_ids: Vec<String> = self.storage
            .get_json(&deliberations_key)
            .unwrap_or_else(|_| Vec::new());
        
        deliberation_ids.push(deliberation.id.clone());
        
        match self.storage.put_json(&deliberations_key, &deliberation_ids) {
            Ok(_) => println!("Successfully updated deliberation list"),
            Err(e) => {
                println!("Error updating deliberation list: {:?}", e);
                return Err(e);
            }
        }
        
        // Calculate quality score for deliberation
        let comment_length = comment.len();
        let reference_count = references.len();
        
        // Simple scoring based on length and references
        let quality_score = if comment_length > 500 {
            0.9 // High quality
        } else if comment_length > 200 {
            0.7 // Medium quality
        } else if comment_length > 50 {
            0.5 // Basic quality
        } else {
            0.3 // Low quality
        };
        
        // Bonus for references
        let reference_bonus = (reference_count as f64 * 0.1).min(0.3);
        let final_score = (quality_score + reference_bonus).min(1.0);
        
        // Create evidence for reputation
        if let Some(reputation) = &self.reputation {
            let evidence_data = json!({
                "proposal_id": proposal_id,
                "deliberation_id": deliberation.id,
                "comment_length": comment_length,
                "reference_count": reference_count,
                "quality_score": final_score
            });
            
            // Create evidence object
            let evidence_obj = crate::reputation::Evidence {
                evidence_type: "deliberation".to_string(),
                evidence_id: deliberation.id.clone(),
                description: format!("Added deliberation to proposal: {}", proposal_id),
                timestamp: now,
                data: Some(evidence_data.clone()),
            };
            
            // Record this deliberation for reputation
            println!("Creating attestation for deliberation quality");
            match reputation.attestation_manager().create_attestation(
                &self.identity.did,
                AttestationType::GovernanceQuality,
                final_score,
                evidence_data,
                vec![evidence_obj],
                1,
                Some(150), // Valid for 150 days
            ) {
                Ok(_) => println!("Successfully created deliberation quality attestation"),
                Err(e) => println!("Error creating deliberation quality attestation: {:?}", e),
            }
        }
        
        Ok(deliberation)
    }
    
    // Get all deliberations for a proposal
    pub fn get_deliberations(
        &self,
        proposal_id: &str,
    ) -> Result<Vec<Deliberation>, Box<dyn Error>> {
        // Get list of deliberation IDs
        let deliberations_key = format!("proposal_deliberations/{}", proposal_id);
        println!("Looking for deliberations at key: {}", deliberations_key);
        let deliberation_ids: Vec<String> = self.storage
            .get_json(&deliberations_key)
            .unwrap_or_else(|_| Vec::new());
        
        println!("Found {} deliberation IDs", deliberation_ids.len());
        
        // Load each deliberation
        let mut deliberations = Vec::new();
        for id in deliberation_ids {
            let key = format!("deliberations/{}/{}", proposal_id, id);
            println!("Loading deliberation from: {}", key);
            match self.storage.get_json(&key) {
                Ok(deliberation) => {
                    let deliberation: Deliberation = deliberation;
                    deliberations.push(deliberation);
                    println!("Successfully loaded deliberation");
                },
                Err(e) => {
                    println!("Error loading deliberation: {:?}", e);
                    return Err(e);
                }
            }
        }
        
        Ok(deliberations)
    }
    
    // Calculate governance participation score for a member
    pub async fn calculate_governance_score(
        &self,
        member_did: &str,
    ) -> Result<GovernanceParticipationScore, Box<dyn Error>> {
        // Get all proposals - use the Storage object's list method
        let proposal_ids: Vec<String> = self.storage.list("proposals/")
            .unwrap_or_else(|_| Vec::new());
        
        let mut total_proposals = 0;
        let mut proposals_voted = 0;
        let mut proposals_created = 0;
        let mut deliberations_count = 0;
        
        // Collect stats about participation
        for proposal_id in &proposal_ids {
            let proposal: Proposal = self.storage.get_json(&format!("proposals/{}", proposal_id))?;
            
            // Only count completed proposals
            if proposal.status == ProposalStatus::Passed || proposal.status == ProposalStatus::Failed {
                total_proposals += 1;
                
                // Check if member created this proposal
                if proposal.created_by == member_did {
                    proposals_created += 1;
                }
                
                // Check if member voted on this proposal
                if proposal.votes.iter().any(|v| v.member_did == member_did) {
                    proposals_voted += 1;
                }
                
                // Count deliberations
                let deliberations = self.get_deliberations(proposal_id)?;
                deliberations_count += deliberations.iter()
                    .filter(|d| d.member_did == member_did)
                    .count();
            }
        }
        
        // Calculate participation percentages
        let vote_participation = if total_proposals > 0 {
            proposals_voted as f64 / total_proposals as f64
        } else {
            0.0
        };
        
        // Calculate overall score
        // 60% weight on voting, 20% on proposing, 20% on deliberation
        let mut score = vote_participation * 0.6;
        
        // Add proposal creation component (max 5 proposals get full credit)
        score += (proposals_created as f64 / 5.0).min(1.0) * 0.2;
        
        // Add deliberation component (max 10 deliberations get full credit)
        score += (deliberations_count as f64 / 10.0).min(1.0) * 0.2;
        
        // Create the score object
        let governance_score = GovernanceParticipationScore {
            member_did: member_did.to_string(),
            total_proposals,
            proposals_voted,
            proposals_created,
            deliberations_count,
            vote_participation,
            overall_score: score,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)?
                .as_secs(),
        };
        
        // If reputation system is available, create an attestation based on this score
        if let Some(reputation) = &self.reputation {
            let evidence = vec![
                Evidence {
                    evidence_type: "governance_participation".to_string(),
                    evidence_id: format!("gov_score:{}:{}", member_did, governance_score.timestamp),
                    description: format!("Governance participation score: {:.2}", score),
                    timestamp: governance_score.timestamp,
                    data: Some(serde_json::to_value(&governance_score)?),
                }
            ];
            
            // Create attestation for overall governance participation
            let _ = reputation.attestation_manager().create_attestation(
                member_did,
                AttestationType::GovernanceQuality,
                score, // Use the calculated score directly
                serde_json::json!({
                    "action": "governance_participation",
                    "vote_rate": vote_participation,
                    "proposals_created": proposals_created,
                    "deliberations": deliberations_count,
                }),
                evidence,
                1,
                Some(180), // Valid for 180 days
            );
        }
        
        Ok(governance_score)
    }
}

// Deliberation structure for comments and discussion on proposals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deliberation {
    pub id: String,
    pub proposal_id: String,
    pub member_did: String,
    pub comment: String,
    pub timestamp: u64,
    pub references: Vec<String>, // References to other comments or evidence
    pub signature: Vec<u8>,
}

// Governance participation score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceParticipationScore {
    pub member_did: String,
    pub total_proposals: usize,
    pub proposals_voted: usize,
    pub proposals_created: usize,
    pub deliberations_count: usize,
    pub vote_participation: f64,
    pub overall_score: f64,
    pub timestamp: u64,
} 