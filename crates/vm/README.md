# ICN Governance Virtual Machine

The ICN Governance Virtual Machine (VM) is a secure, deterministic execution environment for the ICN Governance DSL. It provides a sandboxed, reproducible environment for executing governance operations, proposal voting, and economic transactions.

## Overview

The VM is designed to:

1. **Execute Governance Rules** - Safely execute the actions defined in governance primitives
2. **Manage State** - Store and update cooperative state like roles, proposals, and membership information
3. **Process Voting** - Handle voting logic, quorum checking, and proposal execution
4. **Enforce Permissions** - Ensure actions can only be performed by authorized members
5. **Track Economic Transactions** - Manage asset transfers and credit system operations

## Architecture

The VM has a modular architecture with the following components:

### VMState

The state holds all runtime information, including:

- Roles and permissions
- Membership configurations
- Federation definitions
- Asset registries
- Credit system rules
- Proposal status and votes
- Member records

### Function Registry

The VM maintains a registry of built-in functions that can be called from DSL code, such as:

- `allocateFunds` - Allocate funds from a budget
- `notifyMembers` - Send notifications to members
- `addMember` - Add a new member
- `assignRole` - Assign a role to a member
- `transferAsset` - Transfer assets between accounts

### Execution Engine

The execution engine handles the processing of DSL AST nodes and executing them against the VM state.

## Using the VM

### Basic Usage

```rust
use icn_dsl::{ICNParser, ASTNode};
use icn_vm::VM;

// Initialize VM
let vm = VM::new();

// Parse DSL content
let ast_nodes = ICNParser::parse_file(&dsl_content)?;

// Process nodes in order
for node in ast_nodes {
    vm.execute(node).await?;
}
```

### Member Management

```rust
use icn_vm::{VM, Member};

let vm = VM::new();

// Add a member
let member = Member {
    id: "member1".to_string(),
    did: "did:icn:member1".to_string(),
    name: "Alice".to_string(),
    roles: vec!["Member".to_string()],
    joined_date: "2023-03-01".to_string(),
    credentials: Default::default(),
    attributes: Default::default(),
};

vm.add_member(member).await?;

// Update a member
// ...

// Remove a member
vm.remove_member("member1").await?;
```

### Voting on Proposals

```rust
use icn_vm::{VM, Vote, VoteValue};

// Cast a vote
let vote = Vote {
    member_id: "member1".to_string(),
    proposal_id: "ResourceAllocation".to_string(),
    vote: VoteValue::Yes,
    timestamp: "2023-03-15T14:30:00Z".to_string(),
    weight: 1.0,
};

vm.cast_vote(vote).await?;
```

## Security Model

The VM implements several security features:

1. **Sandboxed Execution** - Functions execute in a controlled environment with restricted access
2. **Permission Checking** - Actions are validated against member roles and permissions
3. **Input Validation** - All inputs are validated before processing
4. **Deterministic Execution** - Given the same inputs, the VM will always produce the same outputs
5. **Auditable Operations** - All operations can be traced and verified

## Extending the VM

The VM can be extended in several ways:

### Custom Functions

Add new functions to the function registry:

```rust
vm.functions.insert(
    "customFunction".to_string(),
    Box::new(|args| {
        // Custom function logic
        Ok(Value::Boolean(true))
    }),
);
```

### Custom State Management

Add additional state to the VM:

```rust
// Define custom state
struct CustomState {
    // ...
}

// Add to VM
vm.state.store.insert("custom_state".to_string(), CustomState::serialize());
```

## Future Enhancements

Planned enhancements for the VM include:

1. **Formal Verification** - Prove correctness of VM operations
2. **Gas Metering** - Measure and limit computational resources
3. **Snapshot and Rollback** - Support for state snapshots and transaction rollback
4. **Plugin System** - Modular extension mechanism for custom behaviors
5. **Distributed Execution** - Support for cooperative distributed execution

## Examples

See the `examples` directory for complete examples of VM usage with the governance DSL:

- `governance_test.rs` - A complete example of parsing and executing governance rules 