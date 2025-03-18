// Example governance DSL script

// Define a proposal
proposal "CommunityExpansion" {
    title: "Expand the community"
    description: "Increase membership and resources"
    voting_method: "majority"
    quorum: 60%
    
    // Log messages help with debugging and auditing
    log "Proposal initiated for community expansion"
}

// Define an asset
asset "CommunityToken" {
    name: "Community Token"
    symbol: "COM"
    initial_supply: 10000
    
    log "Community token created"
}

// Create a transaction
transaction "InitialAllocation" {
    from: "treasury"
    to: "community_fund"
    amount: 5000
    asset: "CommunityToken"
    
    log "Initial allocation of tokens completed"
}

// Another simple proposal
proposal "NetworkUpgrade" {
    title: "Upgrade Network Infrastructure"
    description: "Improve reliability and performance"
    voting_method: "consensus"
    quorum: 75%
    
    log "Network upgrade proposal created"
}

// Final log message
log "DSL script execution completed" 