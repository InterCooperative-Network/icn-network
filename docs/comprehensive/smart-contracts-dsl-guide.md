# ICN Smart Cooperative Contracts and Domain-Specific Language (DSL)

This guide provides comprehensive documentation for the Smart Cooperative Contracts system and its Domain-Specific Language within the ICN Network.

## Overview

The ICN Smart Cooperative Contracts system enables cooperatives to codify agreements, governance rules, economic relationships, and resource allocation policies in a secure, automated, and decentralized manner. Unlike traditional blockchain-based smart contracts, ICN's cooperative contracts are specifically designed for the needs of cooperatives with democratic governance and mutual aid principles.

## Architecture

```
┌───────────────────────────────────────────────────────────────────┐
│                  Smart Cooperative Contract System                │
│                                                                   │
│  ┌─────────────┐   ┌─────────────┐   ┌─────────────────────────┐  │
│  │ Governance  │   │   Contract  │   │                         │  │
│  │     DSL     │──▶│  Compiler   │──▶│     Governance VM       │  │
│  │             │   │             │   │                         │  │
│  └─────────────┘   └─────────────┘   └─────────────┬───────────┘  │
│                                                    │              │
│                                                    ▼              │
│  ┌─────────────┐   ┌─────────────┐   ┌─────────────────────────┐  │
│  │  Contract   │   │  Component  │   │                         │  │
│  │  Templates  │──▶│    APIs     │◀──┤   Contract Execution    │  │
│  │             │   │             │   │                         │  │
│  └─────────────┘   └──────┬──────┘   └─────────────────────────┘  │
│                           │                                        │
└───────────────────────────┼────────────────────────────────────────┘
                            │
              ┌─────────────┼─────────────┐
              ▼             ▼             ▼
┌─────────────────┐ ┌──────────────┐ ┌───────────────┐
│                 │ │              │ │               │
│    Identity     │ │   Economic   │ │  Governance   │
│                 │ │              │ │               │
└─────────────────┘ └──────────────┘ └───────────────┘
```

## 1. Governance DSL

The Governance DSL is a specialized language designed for expressing cooperative governance rules, policies, and agreements.

### 1.1 DSL Syntax

The DSL uses a simple, expressive syntax that's both human-readable and machine-interpretable:

```
// Define a policy
policy "credit_limit_policy" {
  description: "Policy for setting and adjusting credit limits"
  
  // Parameters defined for this policy
  parameters {
    target_cooperative: did,
    credit_limit: number,
    require_approval: boolean = true
  }
  
  // Conditions that must be met for policy execution
  conditions {
    // Only federation admins can set limits above 10000
    if credit_limit > 10000 {
      require caller_has_role("federation_admin")
    } else {
      require caller_has_role("coop_admin") or
             caller_has_role("federation_admin")
    }
    
    // If approval required, create a vote
    if require_approval {
      vote = create_vote({
        title: "Credit limit change for ${target_cooperative}",
        description: "Change credit limit to ${credit_limit}",
        options: ["approve", "reject"],
        voting_period: days(3),
        vote_counting: "simple_majority"
      })
      
      require vote.result == "approve"
    }
  }
  
  // Actions to execute when conditions are met
  actions {
    economic.set_credit_limit(target_cooperative, credit_limit)
    emit event("credit_limit_changed", {
      target: target_cooperative,
      new_limit: credit_limit,
      changed_by: caller_did()
    })
  }
}

// Define a contract for resource sharing agreement
contract "resource_sharing" {
  parameters {
    provider: did,
    consumer: did,
    resource_type: string,
    resource_amount: number,
    compensation: number,
    duration: duration
  }
  
  // State variables for this contract
  state {
    start_time: timestamp,
    is_active: boolean = false,
    usage_counter: number = 0
  }
  
  // Initialize the contract
  initialize {
    state.start_time = now()
    state.is_active = true
  }
  
  // Functions that can be called on this contract
  function use_resource(amount: number) {
    // Only the consumer can call this function
    require caller_did() == consumer
    require state.is_active
    
    // Track usage
    state.usage_counter += amount
    
    // Transfer compensation based on usage
    if amount > 0 {
      economic.transfer(consumer, provider, (compensation / resource_amount) * amount)
    }
    
    emit event("resource_used", {
      consumer: consumer,
      amount: amount,
      remaining: resource_amount - state.usage_counter
    })
  }
  
  // Automatically expire the contract after the duration
  schedule after(duration) {
    state.is_active = false
    emit event("contract_expired", {
      provider: provider,
      consumer: consumer,
      total_used: state.usage_counter
    })
  }
}
```

### 1.2 Language Features

The Governance DSL includes:

- **Policies**: Define governance rules and their enforcement
- **Contracts**: Define agreements between cooperatives
- **Events**: Trigger and respond to system events
- **Voting**: Create and manage democratic decision processes
- **Conditions**: Express complex logical conditions
- **Actions**: Define what happens when conditions are met
- **Scheduled Execution**: Time-based actions and expirations
- **State Management**: Track and update contract state

### 1.3 DSL Core Components

The DSL implementation consists of several core components that work together to provide a powerful and flexible language for expressing cooperative contracts:

#### 1.3.1 Expression Types

The DSL is built around expressions, which are the basic building blocks of the language:

```
┌──────────────────────────────────────────────────────┐
│                    Expressions                       │
├──────────────────────────────────────────────────────┤
│ • Literal         • BinaryOp          • Block        │
│ • Variable        • UnaryOp           • If           │
│ • FunctionCall    • Assignment        • Loop         │
│ • Object          • Array             • PropertyAccess│
│ • IndexAccess                                        │
└──────────────────────────────────────────────────────┘
```

- **Literal**: Direct value representation (e.g., `"hello"`, `42`, `true`)
- **Variable**: Reference to a named value (e.g., `credit_limit`)
- **BinaryOp**: Operations with two operands (e.g., `a + b`, `x > y`)
- **UnaryOp**: Operations with one operand (e.g., `-x`, `!condition`)
- **FunctionCall**: Invoking a function with arguments (e.g., `transfer(from, to, amount)`)
- **Block**: A sequence of expressions executed in order
- **If**: Conditional expression with then/else branches
- **Loop**: Repeating execution based on a condition
- **Assignment**: Assigning a value to a variable (e.g., `x = 42`)
- **Object**: Creating key-value maps (e.g., `{key: value, other: value2}`)
- **Array**: Creating ordered collections (e.g., `[1, 2, 3]`)
- **PropertyAccess**: Accessing object properties (e.g., `person.name`)
- **IndexAccess**: Accessing array elements (e.g., `list[0]`)

#### 1.3.2 Value Types

The DSL supports a range of value types to represent different kinds of data:

```
┌──────────────────────────────────────────────────────┐
│                    Value Types                       │
├──────────────────────────────────────────────────────┤
│ • String          • Number           • Integer       │
│ • Boolean         • Object           • Array         │
│ • Null                                               │
└──────────────────────────────────────────────────────┘
```

- **String**: Text values enclosed in quotes (e.g., `"cooperative"`)
- **Number**: Floating-point numerical values (e.g., `3.14`)
- **Integer**: Whole number values (e.g., `42`)
- **Boolean**: Truth values (`true` or `false`)
- **Object**: Key-value collections (e.g., `{name: "Cooperative A", members: 50}`)
- **Array**: Ordered collections of values (e.g., `[1, 2, 3]`)
- **Null**: Represents the absence of a value

#### 1.3.3 Operators

The DSL supports various operators for expressions:

**Binary Operators**:
- Arithmetic: `+`, `-`, `*`, `/`, `%` (modulo)
- Comparison: `==`, `!=`, `<`, `<=`, `>`, `>=`
- Logical: `&&` (and), `||` (or)

**Unary Operators**:
- `-` (numeric negation)
- `!` (logical not)

#### 1.3.4 Statements and Scripts

Statements are the fundamental units of execution in the DSL:

- A **Statement** contains an expression and optional location information
- A **Script** is a collection of statements forming a complete program
- Location information helps with error reporting and debugging

## 2. DSL Implementation Components

### 2.1 Parser

The DSL Parser transforms source code into an abstract syntax tree (AST) for execution:

```
┌───────────┐      ┌───────────┐      ┌───────────┐      ┌───────────┐
│   Source  │      │ Tokenizer │      │  Parser   │      │    AST    │
│   Code    │─────▶│  (Lexer)  │─────▶│           │─────▶│           │
│           │      │           │      │           │      │           │
└───────────┘      └───────────┘      └───────────┘      └───────────┘
```

The parser handles:

- **Tokenization**: Breaking source code into tokens
- **Syntax Analysis**: Checking for syntax correctness
- **AST Construction**: Building the abstract syntax tree
- **Error Reporting**: Providing helpful error messages with location information

The parser maintains source location information, tracking:
- File name
- Line and column of start position
- Line and column of end position

This allows precise error reporting during both compilation and runtime.

### 2.2 Interpreter

The DSL Interpreter executes the parsed code within an execution environment:

```
┌───────────┐      ┌───────────┐      ┌───────────┐
│    AST    │      │Interpreter│      │Evaluation │
│           │─────▶│           │─────▶│  Result   │
│           │      │           │      │           │
└───────────┘      └───────────┘      └───────────┘
```

Key components of the interpreter:

- **Environment**: Manages variables and their scopes
- **Expression Evaluation**: Computes the result of expressions
- **Standard Library**: Provides built-in functions
- **Error Handling**: Manages runtime errors

#### 2.2.1 Environment

The environment handles variable scoping and lookup:

- **Lexical Scoping**: Variables are looked up in the current scope, then parent scopes
- **Nested Environments**: Functions create child environments for local variables
- **Global Environment**: Contains built-in functions and globals

```
┌───────────────────────────────────────┐
│           Global Environment          │
│                                       │
│ ┌───────────────────────────────────┐ │
│ │        Function Environment       │ │
│ │                                   │ │
│ │ ┌───────────────────────────────┐ │ │
│ │ │      Block Environment        │ │ │
│ │ │                               │ │ │
│ │ └───────────────────────────────┘ │ │
│ └───────────────────────────────────┘ │
└───────────────────────────────────────┘
```

#### 2.2.2 Standard Library

The interpreter includes a standard library with built-in functions:

- **Utility Functions**: `print()`, `len()`, `now()`
- **Type Conversions**: `to_string()`, `to_number()`, `to_boolean()`
- **Collection Operations**: `keys()`, `values()`, `has_key()`
- **Time Functions**: `now()`, `days()`, `hours()`, `minutes()`

### 2.3 Compiler

For performance-critical contracts, the DSL Compiler transforms DSL code into bytecode:

```
┌───────────┐      ┌───────────┐      ┌───────────┐      ┌───────────┐
│    AST    │      │ Optimizer │      │ Bytecode  │      │ Executable│
│           │─────▶│           │─────▶│ Generator │─────▶│ Bytecode  │
│           │      │           │      │           │      │           │
└───────────┘      └───────────┘      └───────────┘      └───────────┘
```

The compiler performs:

- **Optimization**: Simplifies the AST for more efficient execution
- **Type Checking**: Verifies type correctness where possible
- **Bytecode Generation**: Transforms the AST into VM bytecode
- **Validation**: Ensures the contract follows security best practices

### 2.4 DSL Manager

The DSL Manager provides a unified interface for working with DSL scripts:

```
┌───────────────────────────────────────────────────────┐
│                     DSL Manager                       │
├───────────────────────────────────────────────────────┤
│ • Parse scripts                                       │
│ • Execute scripts                                     │
│ • Compile scripts                                     │
│ • Manage templates                                    │
│ • Execute from templates                              │
└───────────────────────────────────────────────────────┘
```

The manager coordinates between the parser, interpreter, compiler, and template engine to provide a cohesive development experience.

### 2.5 Template Engine

The Template Engine facilitates the creation of contracts from templates:

```
┌───────────┐      ┌───────────┐      ┌───────────┐      ┌───────────┐
│  Template │      │ Parameter │      │  Template │      │  Script   │
│ Definition│─────▶│ Validation│─────▶│Instantiation─────▶│           │
│           │      │           │      │           │      │           │
└───────────┘      └───────────┘      └───────────┘      └───────────┘
```

The template engine:

- **Stores Templates**: Maintains a repository of template definitions
- **Validates Parameters**: Ensures required parameters are provided
- **Substitutes Parameters**: Replaces placeholders with parameter values
- **Generates Scripts**: Creates executable scripts from templates

#### 2.5.1 Template Structure

Templates are defined with:

- **Name**: Unique identifier for the template
- **Description**: Human-readable explanation of the template's purpose
- **Parameters**: Definitions of required and optional parameters
- **Script Template**: The parameterized script with placeholders
- **Documentation**: Detailed usage information
- **Tags**: Categories for organization and discovery

#### 2.5.2 Parameter Types

Templates support various parameter types:

- **Basic Types**: String, Number, Integer, Boolean
- **Complex Types**: Object, Array
- **Special Types**: Address (DID), Date, Select (options)

## 3. DSL Compiler

The DSL Compiler transforms human-readable DSL code into bytecode that can be executed by the Governance VM.

### 3.1 Compilation Process

```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│    Parse    │    │  Semantic   │    │   Code      │    │  Bytecode   │
│    Source   │───▶│  Analysis   │───▶│  Generation │───▶│  Output     │
│             │    │             │    │             │    │             │
└─────────────┘    └─────────────┘    └─────────────┘    └─────────────┘
```

### 3.2 Using the Compiler

```bash
# Compile a DSL file
icn-cli contracts compile --source resource_sharing.dsl --output resource_sharing.icnbc

# Compile and deploy in one step
icn-cli contracts deploy --source credit_limit_policy.dsl --params '{"target_cooperative": "did:icn:coop1", "credit_limit": 5000}'
```

## 4. Governance VM

The Governance VM is a secure, sandboxed execution environment for running cooperative contracts.

### 4.1 VM Architecture

The VM consists of:

- **Bytecode Interpreter**: Executes compiled contract bytecode
- **Security Sandbox**: Restricts contract actions to prevent abuse
- **State Management**: Manages and persists contract state
- **Event System**: Handles contract events and triggers
- **Component APIs**: Provides controlled access to system components

### 4.2 VM Security Model

The VM employs a robust security model:

- **Capability-Based Security**: Contracts only access what they're explicitly granted
- **Resource Limits**: Prevents excessive resource consumption
- **Permission Verification**: Checks caller permissions before sensitive operations
- **Formal Verification**: Critical contracts can be formally verified
- **Audit Logging**: All contract executions are logged for accountability

## 5. Contract Templates

The system includes standard templates for common cooperative arrangements.

### 5.1 Available Templates

```bash
# List available contract templates
icn-cli contracts list-templates

# Create a contract from a template
icn-cli contracts create-from-template --template mutual_credit_agreement --params template_params.json
```

### 5.2 Core Templates

- **Mutual Credit Agreement**: Set up economic relationships between cooperatives
- **Resource Sharing**: Define terms for sharing physical or digital resources
- **Federation Membership**: Establish federation relationships and obligations
- **Dispute Resolution**: Set procedures for resolving conflicts
- **Collective Decision**: Define decision-making processes for specific domains

### 5.3 Template Examples

#### Mutual Credit Agreement Template

```json
{
  "name": "MutualCreditAgreement",
  "description": "Establishes a mutual credit line between two cooperatives",
  "parameters": [
    {
      "name": "coop1_did",
      "param_type": "Address",
      "description": "DID of the first cooperative",
      "required": true
    },
    {
      "name": "coop2_did",
      "param_type": "Address",
      "description": "DID of the second cooperative",
      "required": true
    },
    {
      "name": "credit_limit",
      "param_type": "Number",
      "description": "Maximum credit limit for the agreement",
      "required": true
    },
    {
      "name": "duration_days",
      "param_type": "Integer",
      "description": "Duration of the agreement in days",
      "required": true,
      "default_value": 365
    }
  ],
  "script_template": "contract \"mutual_credit_{{coop1_did}}_{{coop2_did}}\" {\n  parameters {\n    coop1: \"{{coop1_did}}\",\n    coop2: \"{{coop2_did}}\",\n    limit: {{credit_limit}},\n    duration: {{duration_days}}\n  }\n\n  initialize {\n    economic.create_mutual_credit_line(coop1, coop2, limit)\n    schedule after(days(duration)) {\n      if !is_renewed {\n        expire_agreement()\n      }\n    }\n  }\n\n  // Rest of the template...\n}"
}
```

#### Voting Proposal Template

```json
{
  "name": "VotingProposal",
  "description": "Creates a proposal for democratic decision-making",
  "parameters": [
    {
      "name": "title",
      "param_type": "String",
      "description": "Proposal title",
      "required": true
    },
    {
      "name": "description",
      "param_type": "String",
      "description": "Detailed proposal description",
      "required": true
    },
    {
      "name": "group_did",
      "param_type": "Address",
      "description": "DID of the voting group",
      "required": true
    },
    {
      "name": "voting_method",
      "param_type": "Select",
      "description": "Method for counting votes",
      "required": true,
      "options": ["simple_majority", "super_majority", "consensus", "quadratic"],
      "default_value": "simple_majority"
    },
    {
      "name": "voting_period_days",
      "param_type": "Integer",
      "description": "Voting period in days",
      "required": false,
      "default_value": 7
    }
  ],
  "script_template": "contract \"proposal_{{title}}\" {\n  // Template implementation...\n}"
}
```

## 6. Component APIs

Contracts interact with ICN components through secure, sandboxed APIs.

### 6.1 Identity API

```
// Identity API examples
identity.resolve_did(did) -> DIDDocument
identity.verify_credential(credential) -> boolean
identity.has_role(did, role) -> boolean
identity.create_credential(subject, type, claims) -> Credential
```

### 6.2 Economic API

```
// Economic API examples
economic.get_balance(account) -> number
economic.transfer(from, to, amount) -> boolean
economic.set_credit_limit(account, limit) -> boolean
economic.create_mutual_credit_line(account1, account2, limit) -> boolean
```

### 6.3 Governance API

```
// Governance API examples
governance.create_proposal(title, description, options) -> proposal_id
governance.cast_vote(proposal_id, voter, choice) -> boolean
governance.count_votes(proposal_id) -> VoteResult
governance.has_permission(did, permission) -> boolean
```

### 6.4 Integration with New Components

The DSL provides APIs for interacting with the new components:

#### Zero-Knowledge Proofs API

```
// ZKP API examples
zkp.create_proof_request(predicate_type, params) -> ProofRequest
zkp.verify_proof(proof, challenge) -> boolean
zkp.generate_challenge(context) -> Challenge
zkp.is_eligible_without_revealing(did, criteria) -> boolean
```

#### Sharding API

```
// Sharding API examples
sharding.get_shard_id() -> ShardId
sharding.cross_shard_transaction(txn_data, target_shard) -> TxnId
sharding.is_same_shard(did1, did2) -> boolean
sharding.route_message(message, target_shard) -> MessageId
```

#### DAO Management API

```
// DAO API examples
dao.create_dao(name, founding_members) -> DID
dao.add_member_to_role(dao_did, member_did, role) -> boolean
dao.has_permission(dao_did, member_did, permission) -> boolean
dao.create_proposal(dao_did, title, description) -> ProposalId
```

#### Incentive System API

```
// Incentive API examples
incentives.submit_contribution(contributor, type, description, evidence) -> ContributionId
incentives.verify_contribution(contribution_id, score, comments) -> boolean
incentives.reward_contribution(contribution_id, scheme, reputation) -> RewardDetails
```

#### Proof of Cooperation API

```
// PoC API examples
poc.get_committee() -> [ValidatorDID]
poc.submit_for_consensus(value) -> ConsensusRoundId
poc.is_consensus_reached(round_id) -> boolean
poc.get_consensus_result(round_id) -> ConsensusResult
```

## 7. DSL Development Workflow

### 7.1 Contract Development Process

```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│    Write    │    │    Test     │    │   Deploy    │    │   Monitor   │
│  Contract   │───▶│  Contract   │───▶│  Contract   │───▶│   Contract  │
│             │    │             │    │             │    │             │
└─────────────┘    └─────────────┘    └─────────────┘    └─────────────┘
```

### 7.2 Development Tools

The ICN platform provides tools for DSL development:

- **DSL Editor**: Syntax highlighting and autocompletion
- **Contract Testing Framework**: Unit and integration testing
- **Contract Debugger**: Step-through execution
- **Contract Explorer**: Browsing deployed contracts
- **Template Designer**: Creating and testing templates

### 7.3 Best Practices

- **Modularize**: Break complex contracts into smaller, reusable components
- **Test Thoroughly**: Cover all edge cases and failure scenarios
- **Use Templates**: Start with templates for common patterns
- **Follow Patterns**: Use established design patterns for contracts
- **Document Clearly**: Add comments and documentation for all contracts
- **Consider Security**: Follow security best practices to prevent vulnerabilities

## 8. Integration Examples

### 8.1 Resource Sharing with Zero-Knowledge Proofs

```
contract "private_resource_sharing" {
  parameters {
    provider: did,
    consumer: did,
    resource_type: string,
    minimum_reputation: number
  }
  
  function request_access() {
    // Only the consumer can request access
    require caller_did() == consumer
    
    // Create a proof request for reputation >= minimum
    var proof_request = zkp.create_proof_request(
      "reputation_threshold", 
      {
        "threshold": minimum_reputation,
        "context": "ResourceSharing"
      }
    )
    
    // Consumer generates proof off-chain
    
    // Verify the proof without revealing actual reputation
    var access_approved = zkp.verify_proof(received_proof, proof_request)
    
    if (access_approved) {
      grant_access(consumer, resource_type)
    }
  }
}
```

### 8.2 Cross-Shard DAO Voting

```
contract "federation_vote" {
  parameters {
    federation_did: did,
    proposal_title: string,
    proposal_description: string
  }
  
  function cast_vote(choice: string) {
    // Get the shard for this member
    var member_shard = sharding.get_shard_for_did(caller_did())
    var federation_shard = sharding.get_shard_for_did(federation_did)
    
    if (member_shard == federation_shard) {
      // Same shard - direct vote
      governance.cast_vote(proposal_id, caller_did(), choice)
    } else {
      // Cross-shard vote
      var vote_txn = {
        "proposal_id": proposal_id,
        "voter": caller_did(),
        "choice": choice
      }
      
      sharding.cross_shard_transaction(vote_txn, federation_shard)
    }
  }
}
```

### 8.3 Incentivized Cooperation

```
contract "cooperative_incentives" {
  parameters {
    cooperative_did: did,
    incentive_scheme: string
  }
  
  function submit_work(work_type: string, evidence: [string]) {
    // Record the contribution
    var contribution_id = incentives.submit_contribution(
      caller_did(),
      work_type,
      "Work contribution to " + cooperative_did,
      evidence
    )
    
    // Get the member's reputation
    var reputation = network.get_peer_reputation(caller_did())
    
    // Schedule verification
    schedule after(days(1)) {
      if (is_verified(contribution_id)) {
        // Reward the contribution
        incentives.reward_contribution(
          contribution_id,
          incentive_scheme,
          reputation
        )
      }
    }
  }
}
```

## 9. Conclusion

The ICN Smart Cooperative Contracts system and its Domain-Specific Language provide a powerful, cooperative-first approach to encoding agreements and governance rules. By combining a user-friendly syntax with cooperative-specific features, the DSL enables cooperatives to express their unique relationships and governance models in code.

With comprehensive integration across all ICN components and dedicated support for advanced features like zero-knowledge proofs, DAOs, incentive mechanisms, and more, the DSL forms the programmable heart of the ICN platform.

Through the template system, cooperatives can quickly create and deploy common agreement types while having the flexibility to customize them for their specific needs, making cooperative automation accessible to all members regardless of technical background. 