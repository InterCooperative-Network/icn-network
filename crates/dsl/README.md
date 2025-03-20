# ICN Governance DSL

The ICN Governance Domain-Specific Language (DSL) is a purpose-built language for defining cooperative governance systems. It provides a human-readable way to express governance structures, economic systems, and federation relationships.

## Overview

The DSL is designed to:

1. **Express Governance Rules** - Define roles, permissions, voting procedures, and proposals
2. **Configure Economic Systems** - Setup mutual credit systems, assets, and resource sharing
3. **Manage Federation Relationships** - Define federations and cross-federation interactions
4. **Establish Membership Structures** - Create onboarding rules and membership types

## Core Primitives

The DSL consists of the following core primitives:

### Roles

Roles define the permissions and capabilities of members within a cooperative.

```
role Admin {
    description = "Administrator role";
    permissions = ["create_proposal", "manage_members", "configure_system"];
    max_members = 5;
    assignable_by = ["Admin"];
}
```

### Membership

Membership primitives define how new members join and what rights they have.

```
membership StandardMembership {
    onboarding = approval_vote;
    default_role = "Member";
    max_members = 200;
    voting_rights = true;
    credentials = ["identity", "membership_voucher"];
}
```

### Assets

Assets represent resources, credits, or other economic tokens within the system.

```
asset CoopCredits {
    type = "mutual_credit";
    description = "Cooperative mutual credit";
    initial_supply = 10000;
    unit = "credit";
    divisible = true;
    permissions = {
        transfer = "Member";
        issue = "Admin";
        receive = "Member";
    };
}
```

### Proposals

Proposals define governance decisions with voting rules and execution paths.

```
proposal ResourceAllocation {
    title = "Allocate Computing Resources";
    description = "Proposal to allocate computing resources";
    quorum = 25%;
    threshold = 50%;
    voting = ranked_choice;
    required_role = "Contributor";
    voting_period = 259200; // 3 days in seconds
    
    execution = {
        transferAsset("ComputePoolMain", "EducationProject", 200);
        notifyMembers("Resources allocated to project");
    }
}
```

### Federation

Federations define relationships between cooperatives.

```
federation LocalCooperative {
    name = "Local Cooperative Alliance";
    description = "A federation of local worker cooperatives";
    governance_model = "democratic";
    members = ["coop1", "coop2", "coop3"];
    resources = ["compute_cluster", "storage_pool"];
}
```

### Credit System

Credit systems define the economic model and rules for mutual credit.

```
credit_system StandardCredit {
    type = "mutual_credit";
    default_limit = 1000;
    global_limit = 100000;
    limit_calculation = "reputation_based";
    trust_metric = "contribution_history";
}
```

## Voting Methods

The DSL supports various voting methods:

- `majority` - Simple majority voting
- `consensus` - Requires high agreement threshold
- `ranked_choice` - Ranked choice voting
- `quadratic` - Quadratic voting (votes weighted as square root)
- `single_choice` - Single choice from multiple options

## Using the DSL

### Parsing DSL Files

```rust
use icn_dsl::{ICNParser, ASTNode};
use std::fs;

// Load DSL content
let dsl_content = fs::read_to_string("governance.icndsl")?;

// Parse into AST nodes
let ast_nodes = ICNParser::parse_file(&dsl_content)?;

// Process nodes
for node in ast_nodes {
    match node {
        ASTNode::Proposal(proposal) => println!("Found proposal: {}", proposal.title),
        ASTNode::Role(role) => println!("Found role: {}", role.name),
        // ...
    }
}
```

### Executing in the VM

The DSL is typically executed within the ICN Virtual Machine (VM):

```rust
use icn_dsl::{ICNParser, ASTNode};
use icn_vm::VM;

// Parse DSL content
let ast_nodes = ICNParser::parse_file(&dsl_content)?;

// Initialize VM
let vm = VM::new();

// Execute nodes
for node in ast_nodes {
    vm.execute(node).await?;
}
```

## Grammar

The DSL uses a formal grammar defined using the Pest parser generator. The grammar specifies the syntax for each primitive and ensures that DSL files are properly structured.

## Examples

See the `examples` directory for complete examples of DSL usage:

- `governance_example.icndsl` - A comprehensive example showing all primitives
- `governance_test.rs` - A Rust example showing how to parse and execute DSL files 