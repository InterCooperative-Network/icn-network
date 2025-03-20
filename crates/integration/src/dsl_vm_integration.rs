//! Integration tests for the DSL and VM
//!
//! This module tests the integration between the DSL parser and VM execution
//! to ensure they work together correctly for governance operations.

use std::collections::HashMap;
use std::error::Error;
use std::path::Path;
use std::fs;

use icn_dsl::{
    ICNParser, ASTNode, Proposal, Asset, Role, Membership, 
    Federation, CreditSystem, VotingMethod, ExecutionStep, Value,
    OnboardingMethod
};
use icn_vm::{VM, Member, Vote, VoteValue};

/// Test helper to create a basic proposal
fn create_test_proposal() -> Proposal {
    Proposal {
        title: "Test Proposal".to_string(),
        description: "A test proposal for unit testing".to_string(),
        quorum: 50.0,
        threshold: Some(50.0),
        voting_method: VotingMethod::Majority,
        required_role: None,
        voting_period: Some(86400), // 1 day in seconds
        category: Some("test".to_string()),
        tags: Some(vec!["test".to_string(), "proposal".to_string()]),
        execution: vec![
            ExecutionStep {
                function: "notifyMembers".to_string(),
                args: vec![Value::String("Proposal executed".to_string())],
            },
        ],
        rejection: Some(vec![
            ExecutionStep {
                function: "notifyMembers".to_string(),
                args: vec![Value::String("Proposal rejected".to_string())],
            },
        ]),
    }
}

/// Test helper to create a test role
fn create_test_role() -> Role {
    Role {
        name: "TestRole".to_string(),
        description: Some("A test role".to_string()),
        permissions: vec!["create_proposal".to_string(), "vote".to_string()],
        parent_role: None,
        max_members: Some(10),
        assignable_by: None,
        attributes: HashMap::new(),
    }
}

/// Test helper to create a test asset
fn create_test_asset() -> Asset {
    Asset {
        name: "TestAsset".to_string(),
        asset_type: "token".to_string(),
        description: Some("A test asset".to_string()),
        initial_supply: 1000.0,
        unit: Some("TEST".to_string()),
        divisible: Some(true),
        permissions: HashMap::new(),
    }
}

/// Test helper to create a test membership
fn create_test_membership() -> Membership {
    Membership {
        name: "TestMembership".to_string(),
        onboarding: OnboardingMethod::ApprovalVote,
        default_role: Some("TestRole".to_string()),
        max_members: Some(100),
        voting_rights: Some(true),
        credentials: None,
        attributes: HashMap::new(),
    }
}

/// Test helper to create a test member
fn create_test_member(id: &str, name: &str, roles: Vec<&str>) -> Member {
    Member {
        id: id.to_string(),
        did: format!("did:icn:{}", id),
        name: name.to_string(),
        roles: roles.iter().map(|&r| r.to_string()).collect(),
        joined_date: "2023-03-01".to_string(),
        credentials: HashMap::new(),
        attributes: HashMap::new(),
    }
}

#[tokio::test]
async fn test_basic_proposal_execution() -> Result<(), Box<dyn Error>> {
    // Initialize the VM
    let vm = VM::new();
    
    // Create and execute a proposal
    let proposal = create_test_proposal();
    let result = vm.execute(ASTNode::Proposal(proposal)).await?;
    
    // Verify the result
    assert!(matches!(result, Value::Array(_)));
    
    Ok(())
}

#[tokio::test]
async fn test_role_definition() -> Result<(), Box<dyn Error>> {
    // Initialize the VM
    let vm = VM::new();
    
    // Create and execute a role definition
    let role = create_test_role();
    let result = vm.execute(ASTNode::Role(role)).await?;
    
    // Verify the result
    assert!(matches!(result, Value::Boolean(true)));
    assert!(vm.state.roles.contains_key("TestRole"));
    
    Ok(())
}

#[tokio::test]
async fn test_membership_definition() -> Result<(), Box<dyn Error>> {
    // Initialize the VM
    let vm = VM::new();
    
    // Create and execute a membership definition
    let membership = create_test_membership();
    let result = vm.execute(ASTNode::Membership(membership)).await?;
    
    // Verify the result
    assert!(matches!(result, Value::Boolean(true)));
    assert!(vm.state.memberships.contains_key("TestMembership"));
    
    Ok(())
}

#[tokio::test]
async fn test_voting_process() -> Result<(), Box<dyn Error>> {
    // Initialize the VM
    let vm = VM::new();
    
    // Create and add a role
    let role = create_test_role();
    vm.execute(ASTNode::Role(role)).await?;
    
    // Create and add a proposal
    let proposal = create_test_proposal();
    vm.execute(ASTNode::Proposal(proposal.clone())).await?;
    
    // Add members for voting
    let member1 = create_test_member("member1", "Alice", vec!["TestRole"]);
    let member2 = create_test_member("member2", "Bob", vec!["TestRole"]);
    let member3 = create_test_member("member3", "Carol", vec!["TestRole"]);
    
    vm.add_member(member1).await?;
    vm.add_member(member2).await?;
    vm.add_member(member3).await?;
    
    // Cast votes
    let vote1 = Vote {
        member_id: "member1".to_string(),
        proposal_id: proposal.title.clone(),
        vote: VoteValue::Yes,
        timestamp: "2023-03-15T14:30:00Z".to_string(),
        weight: 1.0,
    };
    
    let vote2 = Vote {
        member_id: "member2".to_string(),
        proposal_id: proposal.title.clone(),
        vote: VoteValue::Yes,
        timestamp: "2023-03-15T14:45:00Z".to_string(),
        weight: 1.0,
    };
    
    let vote3 = Vote {
        member_id: "member3".to_string(),
        proposal_id: proposal.title.clone(),
        vote: VoteValue::No,
        timestamp: "2023-03-15T15:00:00Z".to_string(),
        weight: 1.0,
    };
    
    // Cast votes
    vm.cast_vote(vote1).await?;
    vm.cast_vote(vote2).await?;
    vm.cast_vote(vote3).await?;
    
    // The proposal should be executed as majority is reached and quorum is met
    
    Ok(())
}

#[tokio::test]
async fn test_dsl_parse_and_execute() -> Result<(), Box<dyn Error>> {
    // Define a simple DSL script
    let dsl_content = r#"
        role Member {
            description = "Basic member role";
            permissions = ["create_proposal", "vote", "transfer_assets"];
        }
        
        membership StandardMembership {
            onboarding = approval_vote;
            default_role = "Member";
            max_members = 100;
            voting_rights = true;
        }
        
        asset Credits {
            type = "mutual_credit";
            description = "Cooperative credits";
            initial_supply = 1000;
            unit = "credit";
            divisible = true;
        }
        
        proposal TestProposal {
            title = "Test DSL Proposal";
            description = "A proposal to test DSL integration";
            quorum = 50%;
            threshold = 50%;
            voting = majority;
            
            execution = {
                notifyMembers("Proposal from DSL executed");
            }
        }
    "#;
    
    // Parse the DSL
    let ast_nodes = ICNParser::parse_file(dsl_content)?;
    
    // Verify parsing results
    assert_eq!(ast_nodes.len(), 4); // Should have 4 nodes
    
    // Initialize the VM
    let vm = VM::new();
    
    // Execute all nodes
    for node in ast_nodes {
        let result = vm.execute(node).await?;
        assert!(matches!(result, Value::Boolean(true)) || matches!(result, Value::Array(_)));
    }
    
    // Verify state
    assert!(vm.state.roles.contains_key("Member"));
    assert!(vm.state.memberships.contains_key("StandardMembership"));
    assert!(vm.state.assets.contains_key("Credits"));
    assert!(vm.state.proposals.contains_key("Test DSL Proposal"));
    
    Ok(())
}

#[tokio::test]
async fn test_complete_governance_flow() -> Result<(), Box<dyn Error>> {
    // Initialize the VM
    let vm = VM::new();
    
    // 1. Define roles
    let admin_role = Role {
        name: "Admin".to_string(),
        description: Some("Administrator role".to_string()),
        permissions: vec![
            "create_proposal".to_string(),
            "manage_members".to_string(),
            "configure_system".to_string(),
        ],
        parent_role: None,
        max_members: Some(5),
        assignable_by: None,
        attributes: HashMap::new(),
    };
    
    let member_role = Role {
        name: "Member".to_string(),
        description: Some("Regular member".to_string()),
        permissions: vec![
            "create_proposal".to_string(),
            "vote".to_string(),
            "transfer_assets".to_string(),
        ],
        parent_role: None,
        max_members: None,
        assignable_by: Some(vec!["Admin".to_string()]),
        attributes: HashMap::new(),
    };
    
    // 2. Define membership
    let membership = Membership {
        name: "StandardMembership".to_string(),
        onboarding: OnboardingMethod::ApprovalVote,
        default_role: Some("Member".to_string()),
        max_members: Some(100),
        voting_rights: Some(true),
        credentials: None,
        attributes: HashMap::new(),
    };
    
    // 3. Define economic components
    let credits = Asset {
        name: "Credits".to_string(),
        asset_type: "mutual_credit".to_string(),
        description: Some("Cooperative credits".to_string()),
        initial_supply: 10000.0,
        unit: Some("CRED".to_string()),
        divisible: Some(true),
        permissions: {
            let mut perms = HashMap::new();
            perms.insert("transfer".to_string(), Value::String("Member".to_string()));
            perms.insert("issue".to_string(), Value::String("Admin".to_string()));
            perms
        },
    };
    
    // 4. Execute all definitions
    vm.execute(ASTNode::Role(admin_role)).await?;
    vm.execute(ASTNode::Role(member_role)).await?;
    vm.execute(ASTNode::Membership(membership)).await?;
    vm.execute(ASTNode::Asset(credits)).await?;
    
    // 5. Add members
    let admin = create_test_member("admin1", "Admin User", vec!["Admin"]);
    let member1 = create_test_member("member1", "Alice", vec!["Member"]);
    let member2 = create_test_member("member2", "Bob", vec!["Member"]);
    
    vm.add_member(admin).await?;
    vm.add_member(member1).await?;
    vm.add_member(member2).await?;
    
    // 6. Create a proposal
    let proposal = Proposal {
        title: "Allocate Budget".to_string(),
        description: "Allocate budget for the education project".to_string(),
        quorum: 50.0,
        threshold: Some(50.0),
        voting_method: VotingMethod::Majority,
        required_role: Some("Member".to_string()),
        voting_period: Some(86400), // 1 day in seconds
        category: Some("budget".to_string()),
        tags: Some(vec!["education".to_string(), "budget".to_string()]),
        execution: vec![
            ExecutionStep {
                function: "allocateFunds".to_string(),
                args: vec![
                    Value::String("Education".to_string()),
                    Value::Number(500.0),
                ],
            },
            ExecutionStep {
                function: "notifyMembers".to_string(),
                args: vec![Value::String("Budget allocated to education project".to_string())],
            },
        ],
        rejection: None,
    };
    
    vm.execute(ASTNode::Proposal(proposal.clone())).await?;
    
    // 7. Cast votes
    let vote1 = Vote {
        member_id: "admin1".to_string(),
        proposal_id: "Allocate Budget".to_string(),
        vote: VoteValue::Yes,
        timestamp: "2023-03-15T14:30:00Z".to_string(),
        weight: 1.0,
    };
    
    let vote2 = Vote {
        member_id: "member1".to_string(),
        proposal_id: "Allocate Budget".to_string(),
        vote: VoteValue::Yes,
        timestamp: "2023-03-15T14:45:00Z".to_string(),
        weight: 1.0,
    };
    
    let vote3 = Vote {
        member_id: "member2".to_string(),
        proposal_id: "Allocate Budget".to_string(),
        vote: VoteValue::No,
        timestamp: "2023-03-15T15:00:00Z".to_string(),
        weight: 1.0,
    };
    
    // Cast votes
    vm.cast_vote(vote1).await?;
    vm.cast_vote(vote2).await?;
    vm.cast_vote(vote3).await?;
    
    // The proposal should be executed as majority is reached and quorum is met
    
    Ok(())
}

// Integration test with the example file if available
#[tokio::test]
#[ignore] // Ignore by default since the example file path might not exist in CI
async fn test_example_governance_dsl() -> Result<(), Box<dyn Error>> {
    let example_path = Path::new("examples/governance_example.icndsl");
    
    // Skip if the file doesn't exist
    if !example_path.exists() {
        return Ok(());
    }
    
    // Load the example file
    let dsl_content = fs::read_to_string(example_path)?;
    
    // Parse the DSL
    let ast_nodes = ICNParser::parse_file(&dsl_content)?;
    
    // Verify we got some nodes
    assert!(!ast_nodes.is_empty());
    
    // Initialize the VM
    let vm = VM::new();
    
    // Execute all nodes in appropriate order
    
    // First process roles
    for node in ast_nodes.iter().filter(|n| matches!(n, ASTNode::Role(_))) {
        vm.execute(node.clone()).await?;
    }
    
    // Then process memberships
    for node in ast_nodes.iter().filter(|n| matches!(n, ASTNode::Membership(_))) {
        vm.execute(node.clone()).await?;
    }
    
    // Then process federations
    for node in ast_nodes.iter().filter(|n| matches!(n, ASTNode::Federation(_))) {
        vm.execute(node.clone()).await?;
    }
    
    // Then process assets
    for node in ast_nodes.iter().filter(|n| matches!(n, ASTNode::Asset(_))) {
        vm.execute(node.clone()).await?;
    }
    
    // Then process credit systems
    for node in ast_nodes.iter().filter(|n| matches!(n, ASTNode::CreditSystem(_))) {
        vm.execute(node.clone()).await?;
    }
    
    // Finally process proposals
    for node in ast_nodes.iter().filter(|n| matches!(n, ASTNode::Proposal(_))) {
        vm.execute(node.clone()).await?;
    }
    
    // Verify some expected state based on the example file
    assert!(vm.state.roles.contains_key("Admin"));
    assert!(vm.state.roles.contains_key("Member"));
    assert!(vm.state.memberships.contains_key("StandardMembership"));
    
    Ok(())
} 