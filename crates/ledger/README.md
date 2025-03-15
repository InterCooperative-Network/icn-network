# ICN Mutual Credit Ledger

This crate provides a decentralized mutual credit ledger for the InterCooperative Network (ICN). It enables tracking of credit, debit, and account balances between participants in the network.

## Features

- **Mutual Credit Accounting**: Track and manage mutual credit balances between participants
- **Transaction Processing**: Process, validate, and apply various transaction types
- **Account Management**: Create and manage accounts with configurable credit limits
- **Transaction History**: Maintain a complete history of transactions for audit and accountability
- **Multi-Currency Support**: Support for multiple currencies or units of account
- **Clearing Mechanism**: Automatically clear mutual debt between accounts

## Architecture

The ledger is built around these key components:

- **MutualCreditLedger**: The main ledger implementation that tracks accounts and transactions
- **TransactionProcessor**: Validates and processes transactions based on defined rules
- **AccountManager**: Handles account creation and management

## Transaction Types

The ledger supports several transaction types:

- **Transfer**: Move credit from one account to another
- **Issuance**: Create new credit in the system (system-level operation)
- **Clearing**: Clear mutual debt between two accounts
- **AccountCreation**: Create a new account
- **AccountUpdate**: Update account metadata
- **CreditLimitAdjustment**: Change an account's credit limit

## Usage Example

```rust
use std::collections::HashMap;
use std::sync::Arc;

use icn_core::storage::JsonStorage;
use icn_identity::{IdentityManager, IdentityProvider};
use icn_ledger::{
    Ledger, MutualCreditLedger, Account, Transaction, TransactionType, LedgerConfig,
};

async fn example() -> Result<(), Box<dyn std::error::Error>> {
    // Set up storage and identity provider
    let storage = Arc::new(JsonStorage::new("path/to/storage"));
    let identity_provider = Arc::new(IdentityManager::new(storage.clone(), None).await?);
    
    // Create the mutual credit ledger
    let ledger_config = LedgerConfig::default();
    let ledger = MutualCreditLedger::new(
        identity_provider.clone(),
        storage.clone(),
        ledger_config,
    ).await?;
    
    // Create accounts
    let alice_account = ledger.create_account(
        "Alice's Account".to_string(),
        None, // Use default currency
        Some(200.0), // Credit limit
        HashMap::new(),
    ).await?;
    
    let bob_account = ledger.create_account(
        "Bob's Account".to_string(),
        None, // Use default currency
        Some(200.0), // Credit limit
        HashMap::new(),
    ).await?;
    
    // Create a transaction from Alice to Bob
    let transfer = ledger.create_transaction(
        TransactionType::Transfer,
        &alice_account.id,
        Some(&bob_account.id),
        50.0,
        None, // Use account currency
        "Payment for services".to_string(),
        HashMap::new(),
        Vec::new(),
    ).await?;
    
    // Process the transaction
    let processed = ledger.confirm_transaction(&transfer.id).await?;
    
    // Check balances
    let alice_balance = ledger.get_balance(&alice_account.id).await?;
    let bob_balance = ledger.get_balance(&bob_account.id).await?;
    
    println!("Alice's balance: {}", alice_balance);
    println!("Bob's balance: {}", bob_balance);
    
    Ok(())
}
```

## Testing

The crate includes comprehensive tests for all its functionality. To run the tests:

```bash
cargo test -p icn-ledger
```

For examples, see the `examples` directory:

```bash
cargo run -p icn-ledger --example basic_example
``` 