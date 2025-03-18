# ICN Network Domain-Specific Language (DSL)

The ICN Network Domain-Specific Language (DSL) provides a simple, human-readable syntax for expressing cooperative governance rules, economic transactions, and resource allocations. This document explains the DSL features, syntax, and usage.

## Overview

The DSL is designed to facilitate automated governance and resource management in cooperative networks. It allows users to:

1. Define governance proposals and voting mechanisms
2. Express economic transactions and asset management
3. Configure network federations and resource allocations
4. Automate decision-making processes

## Architecture

The DSL system consists of several components:

- **Parser**: Converts DSL scripts into an abstract syntax tree (AST)
- **Virtual Machine (VM)**: Executes the parsed AST in a secure, isolated environment
- **Standard Library**: Provides built-in functions and types for common operations
- **Integration Layer**: Connects the DSL system with the ICN Network components
- **Events System**: Propagates actions and results to the rest of the system

## Basic Syntax

### Comments

```
// This is a single-line comment
```

### Proposals

Proposals are the core mechanism for governance in the DSL.

```
proposal "ProposalName" {
  title: "Proposal Title"
  description: "Proposal Description"
  voting_method: majority | ranked_choice | quadratic
  quorum: 60%
  execution {
    action1("param1", "param2")
    action2("param")
  }
}
```

### Assets

Assets represent resources, tokens, or mutual credit systems in the cooperative.

```
asset "AssetName" {
  type: "mutual_credit" | "token" | "resource"
  initial_supply: 1000
}
```

### Transactions

Transactions transfer assets between cooperative members.

```
transaction {
  from: "member1"
  to: "member2"
  amount: 100
  asset: "AssetName"
}
```

### Federations

Federations define network groupings and their properties.

```
federation "FederationName" {
  bootstrap_peers: ["peer1", "peer2"]
  allow_cross_federation: true | false
  encrypt: true | false
  use_wireguard: true | false
}
```

## CLI Usage

The DSL is integrated into the ICN CLI with the following commands:

```
icn dsl execute-script <file> [--federation <federation>]
icn dsl execute-script-string <script> [--federation <federation>]
icn dsl create-template <template_type> <output>
icn dsl validate <file>
icn dsl show-docs
```

### Examples

#### Creating a Proposal

```
icn dsl create-template governance proposal.dsl
icn dsl execute-script proposal.dsl
```

#### Running an Economic Transaction

```
icn dsl execute-script-string 'transaction { from: "alice", to: "bob", amount: 100, asset: "MutualCredit" }'
```

## Integration with ICN Components

The DSL integrates with:

1. **Governance**: Proposals, voting, and execution
2. **Networking**: Federation configuration and peer management
3. **Storage**: Access control and resource allocation
4. **Identity**: Member authentication and verification

## Security Considerations

- The DSL VM executes in an isolated environment
- Permission checks are enforced for sensitive operations
- Federation boundaries are respected for cross-federation operations
- All operations are logged and can be audited

## Future Enhancements

- Conditional expressions and control flow
- Advanced economic models (demurrage, time-based credits)
- AI-assisted proposal generation
- Multi-signature proposal execution
- Scheduled and recurring actions

## Template Types

The DSL system provides several templates to help users get started:

1. **Governance**: Templates for proposals and voting
2. **Network**: Templates for federation configuration
3. **Economic**: Templates for asset definition and transactions 