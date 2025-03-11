// Basic Majority Voting Policy
policy standard_proposal {
    requires:
        minimum_voters: 10
        approval_threshold: 0.66  # 66% must approve
    applies_to:
        proposal_types: [resource_allocation, membership]
}

// Quadratic Voting Implementation
voting_rule quadratic_voting {
    threshold: 51%
    weighting: sqrt(reputation_score)
    duration: 7d
    quorum: 33%
    applies_to:
        proposal_types: [governance_change, strategic_decision]
}

// Automated Resource Allocation
allocation compute_resources {
    resources: [cpu_hours, gpu_access]
    distribution: reputation_weighted
    recipients: all_active_members
    conditions:
        minimum_participation: 5  # Must have participated in at least 5 activities
        maximum_unutilized: 20%   # No more than 20% previously unused
}

// Reputation Decay Implementation
action reputation_decay {
    trigger: time(weekly)
    effect: 
        adjust_reputation:
            method: multiplicative
            factor: 0.98
    constraints:
        minimum_reputation: 1
}

// Committee Formation and Powers
committee technical_committee {
    size: 5..7  # Between 5 and 7 members
    selection: election
    term: 180d
    powers: [
        software_release_approval,
        technical_standards_creation,
        security_incident_response
    ]
    oversight:
        removal_threshold: 0.75  # 75% of cooperative can remove committee
}

// Multi-stage Proposal Process
process improvement_proposal {
    stages: [
        {
            name: "discussion",
            duration: 7d,
            transition: automatic
        },
        {
            name: "refinement",
            duration: 7d,
            transition: committee_approval
        },
        {
            name: "voting",
            duration: 3d,
            transition: voting_completion
        },
        {
            name: "implementation",
            duration: 30d,
            transition: completion_verification
        }
    ]
    
    requires:
        stage.voting.approval_threshold: 0.6
        stage.implementation.verification: technical_committee
}

// Revenue Sharing Policy
allocation revenue_sharing {
    trigger: event(revenue_received)
    distribution:
        method: proportional
        basis: contribution_hours
        minimum_share: 100  # No member receives less than 100 credits
        maximum_differential: 3.0  # Highest cannot exceed 3x lowest
    exceptions:
        solidarity_fund: 5%  # 5% to solidarity fund
        reserves: 10%  # 10% to cooperative reserves
}

// Conflict Resolution Process
process conflict_resolution {
    stages: [
        {
            name: "direct_dialogue",
            duration: 7d,
            facilitator: none
        },
        {
            name: "mediation",
            duration: 14d,
            facilitator: elected_mediator
        },
        {
            name: "cooperative_council",
            duration: 14d,
            facilitator: council
        }
    ]
    
    outcomes: [
        resolution_agreement,
        binding_decision,
        membership_review
    ]
}

// Privacy-Preserving Voting
voting_rule confidential_voting {
    mechanism: zero_knowledge
    verification: public
    anonymity: full
    prevents: coercion
    applies_to:
        proposal_types: [sensitive_issues, leadership_selection]
}
