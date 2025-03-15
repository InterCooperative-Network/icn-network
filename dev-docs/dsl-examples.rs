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

// Transaction Validation Rule
transaction_rule cooperative_transaction {
    applies_to:
        transaction_types: [resource_exchange, credit_transfer, labor_compensation]
    validation:
        minimum_reputation: 10  # Sender must have at least 10 reputation
        maximum_amount: 5000    # Max 5000 credits per transaction
        daily_limit: 20000      # Max 20000 credits per day
    conditions:
        active_membership: true  # Sender must be active member
        federation_authorized: true  # Transaction must be within authorized federations
    actions_on_validation:
        update_transaction:
            method: add_metadata
            metadata: { cooperative_approved: true, validation_time: current_time() }
        notify_participants: true
    exceptions:
        emergency_override: {
            requires: committee_approval
            committee: emergency_response
            retention: 7d  # Record exception for 7 days
        }
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

// Bylaw Definition Contract
bylaw membership_requirements {
    title: "Membership Requirements"
    version: "1.0"
    effective_date: 2023-05-15
    provisions: [
        {
            id: "min_participation"
            description: "Minimum participation requirements"
            rule: activity_count >= 5 per month
            enforcement: automatic
            consequence: status.set_inactive if rule.violated for 2 months
        },
        {
            id: "ethics_compliance"
            description: "Ethical standards compliance"
            rule: no_violations of ethical_guidelines
            enforcement: review_committee
            consequence: membership_review if rule.violated
        },
        {
            id: "contribution_requirement"
            description: "Regular contribution requirements"
            rule: resource_contribution >= 10 hours per month OR credit_contribution >= 100 per month
            enforcement: automatic
            consequence: status.reduce_benefits if rule.violated
        }
    ]
    amendments: {
        process: standard_voting
        quorum: 40%
        threshold: 66%
    }
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
