use super::primitives::*;
use std::error::Error;
use async_trait::async_trait;

pub struct BasicVotingStrategy;

#[async_trait]
impl VotingStrategy for BasicVotingStrategy {
    async fn tally_votes(&self, votes: &[Vote], method: &VotingMethod) -> Result<ProposalState, Box<dyn Error>> {
        match method.method_type {
            VotingType::Simple => self.tally_simple_votes(votes, &method.threshold),
            VotingType::RankedChoice => self.tally_ranked_choice(votes, method),
            VotingType::Quadratic => self.tally_quadratic(votes, method),
            VotingType::Consensus => self.tally_consensus(votes),
        }
    }
}

impl BasicVotingStrategy {
    fn tally_simple_votes(&self, votes: &[Vote], threshold: &Threshold) -> Result<ProposalState, Box<dyn Error>> {
        let mut yes_votes = 0.0;
        let mut total_votes = 0.0;

        for vote in votes {
            match vote.choice {
                VoteChoice::Yes => {
                    yes_votes += vote.weight;
                    total_votes += vote.weight;
                }
                VoteChoice::No => {
                    total_votes += vote.weight;
                }
                VoteChoice::Abstain => {}
                _ => return Err("Invalid vote choice for simple voting".into()),
            }
        }

        if total_votes == 0.0 {
            return Ok(ProposalState::Active);
        }

        let approval_ratio = yes_votes / total_votes;
        
        match threshold {
            Threshold::Majority => {
                if approval_ratio > 0.5 {
                    Ok(ProposalState::Passed)
                } else {
                    Ok(ProposalState::Rejected)
                }
            }
            Threshold::SuperMajority(required) => {
                if approval_ratio >= *required {
                    Ok(ProposalState::Passed)
                } else {
                    Ok(ProposalState::Rejected)
                }
            }
            Threshold::Unanimous => {
                if approval_ratio == 1.0 {
                    Ok(ProposalState::Passed)
                } else {
                    Ok(ProposalState::Rejected)
                }
            }
            Threshold::Custom(required) => {
                if approval_ratio >= *required {
                    Ok(ProposalState::Passed)
                } else {
                    Ok(ProposalState::Rejected)
                }
            }
        }
    }

    fn tally_ranked_choice(&self, votes: &[Vote], method: &VotingMethod) 
        -> Result<ProposalState, Box<dyn Error>> 
    {
        // Implement Instant Runoff Voting (IRV)
        let num_options = method.options.len();
        let mut eliminated = vec![false; num_options];
        let mut round = 0;

        loop {
            // Count first preferences
            let mut counts = vec![0; num_options];
            for vote in votes {
                if let VoteChoice::Ranked(rankings) = &vote.choice {
                    // Find first non-eliminated preference
                    for &pref in rankings {
                        if !eliminated[pref] {
                            counts[pref] += 1;
                            break;
                        }
                    }
                }
            }

            // Find winner or loser
            let total_votes = counts.iter().sum::<i32>();
            let threshold = (total_votes as f64 / 2.0).ceil() as i32;

            if let Some(winner) = counts.iter()
                .enumerate()
                .filter(|(i, _)| !eliminated[*i])
                .find(|(_, &count)| count >= threshold)
            {
                return Ok(ProposalState::Passed);
            }

            // No winner, eliminate lowest and continue
            if let Some(loser) = counts.iter()
                .enumerate()
                .filter(|(i, _)| !eliminated[*i])
                .min_by_key(|(_, &count)| count)
            {
                eliminated[loser.0] = true;
                round += 1;

                // If we've eliminated all but one, that's our winner
                if eliminated.iter().filter(|&&e| !e).count() == 1 {
                    return Ok(ProposalState::Passed);
                }

                // If we've somehow eliminated everyone, reject
                if eliminated.iter().all(|&e| e) {
                    return Ok(ProposalState::Rejected);
                }
            }
        }
    }

    fn tally_quadratic(&self, votes: &[Vote], method: &VotingMethod) 
        -> Result<ProposalState, Box<dyn Error>> 
    {
        let mut total_score = 0.0;
        
        for vote in votes {
            if let VoteChoice::Quadratic(weights) = &vote.choice {
                // Sum the quadratic costs
                for &weight in weights {
                    // Cost is weight^2, effect is weight
                    total_score += weight as f64;
                }
            }
        }

        // For quadratic voting, we typically use a simple majority
        if total_score > 0.0 {
            Ok(ProposalState::Passed)
        } else {
            Ok(ProposalState::Rejected)
        }
    }

    fn tally_consensus(&self, votes: &[Vote]) -> Result<ProposalState, Box<dyn Error>> {
        // For consensus, any single "No" vote means rejection
        for vote in votes {
            match vote.choice {
                VoteChoice::No => return Ok(ProposalState::Rejected),
                VoteChoice::Abstain => {}
                VoteChoice::Yes => {}
                _ => return Err("Invalid vote choice for consensus voting".into()),
            }
        }

        // If we got here, no one objected
        Ok(ProposalState::Passed)
    }
} 