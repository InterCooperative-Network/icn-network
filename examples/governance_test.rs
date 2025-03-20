use std::error::Error;
use std::fs;
use std::path::Path;
use icn_dsl::{ICNParser, ASTNode};
use icn_vm::VM;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Configure logging
    tracing_subscriber::fmt::init();
    
    println!("ICN Cooperative Governance Example");
    println!("----------------------------------");
    
    // Load the DSL file
    let dsl_path = Path::new("examples/governance_example.icndsl");
    let dsl_content = fs::read_to_string(dsl_path)?;
    
    println!("Parsing DSL file: {}", dsl_path.display());
    
    // Parse the DSL
    let ast_nodes = ICNParser::parse_file(&dsl_content)?;
    
    println!("Parsed {} governance components:", ast_nodes.len());
    
    // Count component types
    let mut role_count = 0;
    let mut membership_count = 0;
    let mut federation_count = 0;
    let mut asset_count = 0;
    let mut credit_system_count = 0;
    let mut proposal_count = 0;
    
    for node in &ast_nodes {
        match node {
            ASTNode::Role(_) => role_count += 1,
            ASTNode::Membership(_) => membership_count += 1,
            ASTNode::Federation(_) => federation_count += 1,
            ASTNode::Asset(_) => asset_count += 1,
            ASTNode::CreditSystem(_) => credit_system_count += 1,
            ASTNode::Proposal(_) => proposal_count += 1,
        }
    }
    
    println!("  Roles: {}", role_count);
    println!("  Memberships: {}", membership_count);
    println!("  Federations: {}", federation_count);
    println!("  Assets: {}", asset_count);
    println!("  Credit Systems: {}", credit_system_count);
    println!("  Proposals: {}", proposal_count);
    
    // Initialize the VM
    println!("\nInitializing governance VM...");
    let vm = VM::new();
    
    // Process nodes in order (certain components should be processed before others)
    println!("\nExecuting governance components:");
    
    // First process roles
    for node in ast_nodes.iter().filter(|n| matches!(n, ASTNode::Role(_))) {
        if let ASTNode::Role(role) = node {
            println!("  Processing role: {}", role.name);
            vm.execute(node.clone()).await?;
        }
    }
    
    // Then process memberships
    for node in ast_nodes.iter().filter(|n| matches!(n, ASTNode::Membership(_))) {
        if let ASTNode::Membership(membership) = node {
            println!("  Processing membership: {}", membership.name);
            vm.execute(node.clone()).await?;
        }
    }
    
    // Then process federations
    for node in ast_nodes.iter().filter(|n| matches!(n, ASTNode::Federation(_))) {
        if let ASTNode::Federation(federation) = node {
            println!("  Processing federation: {}", federation.name);
            vm.execute(node.clone()).await?;
        }
    }
    
    // Then process assets
    for node in ast_nodes.iter().filter(|n| matches!(n, ASTNode::Asset(_))) {
        if let ASTNode::Asset(asset) = node {
            println!("  Processing asset: {}", asset.name);
            vm.execute(node.clone()).await?;
        }
    }
    
    // Then process credit systems
    for node in ast_nodes.iter().filter(|n| matches!(n, ASTNode::CreditSystem(_))) {
        if let ASTNode::CreditSystem(credit_system) = node {
            println!("  Processing credit system: {}", credit_system.name);
            vm.execute(node.clone()).await?;
        }
    }
    
    // Finally process proposals
    for node in ast_nodes.iter().filter(|n| matches!(n, ASTNode::Proposal(_))) {
        if let ASTNode::Proposal(proposal) = node {
            println!("  Processing proposal: {}", proposal.title);
            vm.execute(node.clone()).await?;
        }
    }
    
    println!("\nAll governance components successfully processed!");
    println!("\nSimulating voting on AddNewMember proposal:");
    
    // Create some test members
    let member1 = icn_vm::Member {
        id: "member1".to_string(),
        did: "did:icn:member1".to_string(),
        name: "Alice".to_string(),
        roles: vec!["Member".to_string()],
        joined_date: "2025-01-01".to_string(),
        credentials: Default::default(),
        attributes: Default::default(),
    };
    
    let member2 = icn_vm::Member {
        id: "member2".to_string(),
        did: "did:icn:member2".to_string(),
        name: "Bob".to_string(),
        roles: vec!["Member".to_string()],
        joined_date: "2025-01-05".to_string(),
        credentials: Default::default(),
        attributes: Default::default(),
    };
    
    let member3 = icn_vm::Member {
        id: "member3".to_string(),
        did: "did:icn:member3".to_string(),
        name: "Carol".to_string(),
        roles: vec!["Admin".to_string()],
        joined_date: "2025-01-10".to_string(),
        credentials: Default::default(),
        attributes: Default::default(),
    };
    
    // Add members to VM
    vm.add_member(member1).await?;
    vm.add_member(member2).await?;
    vm.add_member(member3).await?;
    
    println!("  Added 3 test members to the system");
    
    // Cast votes on the AddNewMember proposal
    let vote1 = icn_vm::Vote {
        member_id: "member1".to_string(),
        proposal_id: "AddNewMember".to_string(),
        vote: icn_vm::VoteValue::Yes,
        timestamp: "2025-03-15T14:30:00Z".to_string(),
        weight: 1.0,
    };
    
    let vote2 = icn_vm::Vote {
        member_id: "member2".to_string(),
        proposal_id: "AddNewMember".to_string(),
        vote: icn_vm::VoteValue::Yes,
        timestamp: "2025-03-15T16:45:00Z".to_string(),
        weight: 1.0,
    };
    
    let vote3 = icn_vm::Vote {
        member_id: "member3".to_string(),
        proposal_id: "AddNewMember".to_string(),
        vote: icn_vm::VoteValue::Yes,
        timestamp: "2025-03-16T09:15:00Z".to_string(),
        weight: 1.0,
    };
    
    println!("  Casting votes from test members:");
    println!("    - Alice: Yes");
    vm.cast_vote(vote1).await?;
    println!("    - Bob: Yes");
    vm.cast_vote(vote2).await?;
    println!("    - Carol: Yes");
    vm.cast_vote(vote3).await?;
    
    println!("\nProposal has been approved and executed!");
    println!("\nDemonstration complete!");
    
    Ok(())
} 