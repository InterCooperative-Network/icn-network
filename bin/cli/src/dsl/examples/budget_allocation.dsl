// Budget Allocation Proposal Example
// This DSL script demonstrates a governance proposal for allocating budget

// Define the proposal
proposal "EducationBudget" {
    title: "Fund Education Program"
    description: "Allocate 500 credits to the Education Program for workshop supplies and speaker fees."
    author: "alice@icn.coop"
    
    // Define voting parameters
    voting {
        method: "ranked_choice"
        threshold: 60%
        quorum: 51%
        duration: "7 days"
    }
    
    // Define what happens when the proposal is approved
    on_approve {
        // Create a transaction to allocate funds
        transaction {
            from: "treasury"
            to: "education_program"
            amount: 500
            asset: "credits"
            memo: "Education Program Funding - Q2 2025"
        }
        
        // Log the allocation
        log("Budget allocation for Education Program approved and executed.")
    }
    
    // Define what happens when the proposal is rejected
    on_reject {
        log("Budget allocation for Education Program was rejected.")
    }
} 