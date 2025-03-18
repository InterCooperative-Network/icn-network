// Federation Definition Example
// This DSL script demonstrates how to define a new federation with roles and permissions

// Define the federation
federation "TechCooperative" {
    name: "Technology Workers Cooperative"
    description: "A federation of technology worker cooperatives focused on open source software development."
    
    // Define membership requirements
    membership {
        type: "application_with_approval"
        approval_threshold: 75%
        min_members: 5
    }
    
    // Define governance structure
    governance {
        decision_method: "consensus"
        fallback_method: "super_majority"
        super_majority_threshold: 66%
        proposal_duration: "14 days"
    }
    
    // Define roles
    role "Member" {
        description: "Regular member of the federation"
        permissions: ["vote", "propose", "join_working_groups"]
    }
    
    role "Facilitator" {
        description: "Facilitates discussions and decision-making processes"
        permissions: ["vote", "propose", "join_working_groups", "facilitate_meetings", "extend_voting"]
        term: "6 months"
        selection: "election"
    }
    
    role "TechnicalCoordinator" {
        description: "Coordinates technical projects and resources"
        permissions: ["vote", "propose", "join_working_groups", "allocate_resources", "approve_technical_decisions"]
        term: "12 months"
        selection: "election"
    }
    
    // Define working groups
    working_group "Infrastructure" {
        description: "Manages shared infrastructure resources"
        roles: ["Member", "TechnicalCoordinator"]
        budget_allocation: 30%
    }
    
    working_group "Education" {
        description: "Organizes educational events and resources"
        roles: ["Member", "Facilitator"]
        budget_allocation: 25%
    }
    
    working_group "Development" {
        description: "Coordinates software development projects"
        roles: ["Member", "TechnicalCoordinator"]
        budget_allocation: 45%
    }
    
    // Define shared resources
    resource "ComputeCluster" {
        type: "compute"
        allocation_method: "fair_share"
        priority_queue: true
    }
    
    resource "SharedStorage" {
        type: "storage"
        allocation_method: "quota"
        default_quota: "500GB"
    }
    
    // Define federation assets
    asset "TechCoin" {
        type: "mutual_credit"
        initial_supply: 10000
        issuance: "democratic"
    }
    
    // On federation creation
    on_create {
        log("Technology Workers Cooperative federation has been created!")
    }
} 