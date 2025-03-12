# ICN Mutual Credit

A mutual credit system implementation for the Intercooperative Network (ICN) project.

## Overview

This crate provides the core functionality for a mutual credit system, a non-extractive economic framework that enables value exchange without requiring a centralized currency. The mutual credit system tracks credits and debits between accounts in a network, with built-in constraints to prevent exploitation.

## Key Components

### Account Management

- `Account`: Represents a participant in the mutual credit system
- Supports different account statuses (Active, Inactive, Suspended, Closed)
- Tracks balance, reputation, and metadata

### Credit Lines

- `CreditLine`: Defines credit relationships between accounts
- Configurable credit limits and terms
- Support for collateral and conditions

### Transactions

- Support for different transaction types:
  - `DirectTransfer`: Simple transfers between directly connected accounts
  - `PathTransfer`: Multi-hop transfers through the credit network
  - `CreditLineAdjustment`: Changes to credit limits
  - `SystemOperation`: Administrative operations

### Credit Graph

- `CreditGraph`: Represents the network of accounts and credit lines
- Provides methods for adding accounts and credit lines
- Handles account and credit line verification

### Transaction Processing

- `TransactionProcessor`: Manages the execution of transactions
- Validates transactions before processing
- Tracks transaction history

### Credit Clearing

- Implements a credit clearing algorithm to reduce circular debt
- Detects cycles in the credit graph
- Creates transactions to offset debt in cycles

## Usage

### Basic Transfer Example

```rust
// Create accounts and credit lines
let coop1_did = DID::new("did:icn:coop:farming-collective");
let coop2_did = DID::new("did:icn:coop:tech-support");

let mut graph = CreditGraph::new();

// Add accounts
graph.add_account(Account::new(coop1_did.clone(), "Farming Collective")).await?;
graph.add_account(Account::new(coop2_did.clone(), "Tech Support")).await?;

// Add credit lines
graph.add_credit_line(CreditLine::new(
    coop1_did.clone(),
    coop2_did.clone(),
    Amount::new(100),
    CreditTerms::new(),
)).await?;

// Create a transaction processor
let graph = Arc::new(Mutex::new(graph));
let mut processor = TransactionProcessor::new(Arc::clone(&graph), None);

// Create and process a transaction
let tx = Transaction::new(
    "tx-001".to_string(),
    coop1_did.clone(),
    coop2_did.clone(),
    Amount::new(30),
    TransactionType::DirectTransfer,
    Some("IT service and maintenance".to_string()),
);

processor.submit_transaction(tx).await?;
let results = processor.process_pending_transactions().await;
```

### Credit Clearing Example

```rust
// Set up a circular credit network
// A -> B -> C -> D -> A

// Create transactions that form a circular debt pattern
let tx1 = Transaction::new(
    "tx-001".to_string(),
    coop_a.clone(),
    coop_b.clone(),
    Amount::new(50),
    TransactionType::DirectTransfer,
    Some("Construction services".to_string()),
);

// ... more transactions ...

// Process transactions
processor.submit_transaction(tx1).await?;
// ... submit more transactions ...
processor.process_pending_transactions().await;

// Run credit clearing algorithm
let clearing_txs = processor.run_credit_clearing().await?;
```

## License

This project is licensed under the MIT License or Apache License 2.0, at your option - see the LICENSE files for details. 