//! Voting schemes for governance
//!
//! This module provides different voting schemes for governance proposals,
//! including simple majority voting and weighted voting based on reputation.

use std::collections::HashSet;
use std::fmt::Debug;
use async_trait::async_trait;
use serde::{Serialize, Deserialize};

use crate::{Vote, GovernanceResult, GovernanceError};

/// Result of a vote tally
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VotingResult {
    /// Whether the proposal was approved
    pub approved: bool,
    /// Whether the vote reached quorum
    pub has_quorum: bool,
    /// Number of yes votes
    pub yes_votes: usize,
    /// Number of no votes
    pub no_votes: usize,
    /// Total votes cast
    pub total_votes: usize,
    /// Percentage of yes votes out of all votes (0.0 to 1.0)
    pub approval_percentage: f64,
    /// Percentage of participation (0.0 to 1.0)
    pub participation_percentage: f64,
    /// Quorum percentage required (0.0 to 1.0)
    pub quorum_percentage: f64,
    /// Approval percentage required (0.0 to 1.0)
    pub approval_percentage_required: f64,
}

/// A trait for different voting schemes
pub trait VotingScheme: Send + Sync + Debug {
    /// Tally votes and determine the result
    fn tally_votes(&self, votes: &[Vote]) -> GovernanceResult<VotingResult>;
}

/// Simple majority voting scheme
#[derive(Debug, Clone)]
pub struct SimpleVoting {
    /// Quorum percentage (0.0 to 1.0)
    quorum_percentage: f64,
    /// Approval percentage required (0.0 to 1.0)
    approval_percentage_required: f64,
}

impl SimpleVoting {
    /// Create a new simple voting scheme
    pub fn new(quorum_percentage: f64, approval_percentage_required: f64) -> Self {
        Self {
            quorum_percentage,
            approval_percentage_required,
        }
    }
}

impl VotingScheme for SimpleVoting {
    fn tally_votes(&self, votes: &[Vote]) -> GovernanceResult<VotingResult> {
        // Deduplicate votes (only count the latest vote from each voter)
        let mut unique_votes = HashMap::new();
        for vote in votes {
            unique_votes.insert(vote.voter.clone(), vote.clone());
        }
        
        let unique_votes: Vec<Vote> = unique_votes.into_values().collect();
        
        let yes_votes = unique_votes.iter().filter(|v| v.approve).count();
        let no_votes = unique_votes.iter().filter(|v| !v.approve).count();
        let total_votes = yes_votes + no_votes;
        
        // In simple voting, we assume maximum participation would be "all eligible voters"
        // For simplicity, we'll estimate this as 100 total potential voters
        // In a real system, this would be the actual count of eligible voters
        let eligible_voters = 100; // Simplification
        
        let participation_percentage = if eligible_voters > 0 {
            total_votes as f64 / eligible_voters as f64
        } else {
            0.0
        };
        
        let approval_percentage = if total_votes > 0 {
            yes_votes as f64 / total_votes as f64
        } else {
            0.0
        };
        
        let has_quorum = participation_percentage >= self.quorum_percentage;
        let meets_approval = approval_percentage >= self.approval_percentage_required;
        
        // A proposal is approved if it meets quorum and approval threshold
        let approved = has_quorum && meets_approval;
        
        Ok(VotingResult {
            approved,
            has_quorum,
            yes_votes,
            no_votes,
            total_votes,
            approval_percentage,
            participation_percentage,
            quorum_percentage: self.quorum_percentage,
            approval_percentage_required: self.approval_percentage_required,
        })
    }
}

/// Reputation-weighted voting scheme
#[derive(Debug, Clone)]
pub struct WeightedVoting {
    /// Quorum percentage (0.0 to 1.0)
    quorum_percentage: f64,
    /// Approval percentage required (0.0 to 1.0)
    approval_percentage_required: f64,
}

impl WeightedVoting {
    /// Create a new weighted voting scheme
    pub fn new(quorum_percentage: f64, approval_percentage_required: f64) -> Self {
        Self {
            quorum_percentage,
            approval_percentage_required,
        }
    }
}

impl VotingScheme for WeightedVoting {
    fn tally_votes(&self, votes: &[Vote]) -> GovernanceResult<VotingResult> {
        // Deduplicate votes (only count the latest vote from each voter)
        let mut unique_votes = HashMap::new();
        for vote in votes {
            unique_votes.insert(vote.voter.clone(), vote.clone());
        }
        
        let unique_votes: Vec<Vote> = unique_votes.into_values().collect();
        
        // Count raw votes for reporting
        let yes_votes_count = unique_votes.iter().filter(|v| v.approve).count();
        let no_votes_count = unique_votes.iter().filter(|v| !v.approve).count();
        let total_votes_count = yes_votes_count + no_votes_count;
        
        // Calculate weighted votes
        let weighted_yes_votes: f64 = unique_votes.iter()
            .filter(|v| v.approve)
            .map(|v| v.weight.unwrap_or(1.0))
            .sum();
        
        let weighted_no_votes: f64 = unique_votes.iter()
            .filter(|v| !v.approve)
            .map(|v| v.weight.unwrap_or(1.0))
            .sum();
        
        let weighted_total = weighted_yes_votes + weighted_no_votes;
        
        // Estimate total potential weighted votes
        // In a real system, this would be calculated based on all eligible voters
        let total_potential_weight = 100.0; // Simplification
        
        let participation_percentage = if total_potential_weight > 0.0 {
            weighted_total / total_potential_weight
        } else {
            0.0
        };
        
        let approval_percentage = if weighted_total > 0.0 {
            weighted_yes_votes / weighted_total
        } else {
            0.0
        };
        
        let has_quorum = participation_percentage >= self.quorum_percentage;
        let meets_approval = approval_percentage >= self.approval_percentage_required;
        
        // A proposal is approved if it meets quorum and approval threshold
        let approved = has_quorum && meets_approval;
        
        Ok(VotingResult {
            approved,
            has_quorum,
            yes_votes: yes_votes_count,
            no_votes: no_votes_count,
            total_votes: total_votes_count,
            approval_percentage,
            participation_percentage,
            quorum_percentage: self.quorum_percentage,
            approval_percentage_required: self.approval_percentage_required,
        })
    }
}

use std::collections::HashMap;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Vote;
    use icn_core::crypto::{NodeId, Signature};
    
    // Helper to create a test vote
    fn create_test_vote(voter_id: &str, approve: bool, weight: Option<f64>) -> Vote {
        Vote {
            proposal_id: "test-proposal".to_string(),
            voter: NodeId::from_string(voter_id),
            approve,
            comment: None,
            weight,
            timestamp: 0,
            signature: Signature(Vec::new()),
        }
    }
    
    #[test]
    fn test_simple_voting_clear_approval() {
        let voting = SimpleVoting::new(0.2, 0.5);
        
        let votes = vec![
            create_test_vote("voter1", true, None),
            create_test_vote("voter2", true, None),
            create_test_vote("voter3", true, None),
            create_test_vote("voter4", false, None),
        ];
        
        let result = voting.tally_votes(&votes).unwrap();
        
        assert_eq!(result.yes_votes, 3);
        assert_eq!(result.no_votes, 1);
        assert_eq!(result.total_votes, 4);
        assert!((result.approval_percentage - 0.75).abs() < 0.001);
        assert!(result.approved);
        assert!(result.has_quorum);
    }
    
    #[test]
    fn test_simple_voting_no_quorum() {
        let voting = SimpleVoting::new(0.3, 0.5);
        
        // With our simplified eligible voters = 100, 2 votes is less than 30% quorum
        let votes = vec![
            create_test_vote("voter1", true, None),
            create_test_vote("voter2", true, None),
        ];
        
        let result = voting.tally_votes(&votes).unwrap();
        
        assert_eq!(result.yes_votes, 2);
        assert_eq!(result.no_votes, 0);
        assert_eq!(result.total_votes, 2);
        assert!((result.approval_percentage - 1.0).abs() < 0.001);
        assert!(!result.approved); // Not approved despite 100% yes, because quorum not met
        assert!(!result.has_quorum);
    }
    
    #[test]
    fn test_weighted_voting() {
        let voting = WeightedVoting::new(0.2, 0.5);
        
        let votes = vec![
            create_test_vote("voter1", true, Some(0.8)),
            create_test_vote("voter2", true, Some(0.6)),
            create_test_vote("voter3", false, Some(0.9)),
            create_test_vote("voter4", false, Some(0.2)),
        ];
        
        let result = voting.tally_votes(&votes).unwrap();
        
        assert_eq!(result.yes_votes, 2);
        assert_eq!(result.no_votes, 2);
        assert_eq!(result.total_votes, 4);
        
        // Weighted yes votes = 0.8 + 0.6 = 1.4
        // Weighted no votes = 0.9 + 0.2 = 1.1
        // Total weighted votes = 2.5
        // Approval percentage = 1.4 / 2.5 = 0.56
        assert!((result.approval_percentage - 0.56).abs() < 0.001);
        assert!(result.approved);
        assert!(result.has_quorum);
    }
    
    #[test]
    fn test_duplicate_voter() {
        let voting = SimpleVoting::new(0.2, 0.5);
        
        // voter1 votes twice, the second vote (no) should override the first
        let votes = vec![
            create_test_vote("voter1", true, None),
            create_test_vote("voter2", true, None),
            create_test_vote("voter1", false, None), // Changed vote
        ];
        
        let result = voting.tally_votes(&votes).unwrap();
        
        assert_eq!(result.yes_votes, 1);
        assert_eq!(result.no_votes, 1);
        assert_eq!(result.total_votes, 2);
        assert!((result.approval_percentage - 0.5).abs() < 0.001);
    }
} 