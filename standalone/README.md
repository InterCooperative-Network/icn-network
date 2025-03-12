# ICN Identity and Mutual Credit Integration Example

This is a simplified standalone example demonstrating the core concepts of the Intercooperative Network (ICN) project, focusing on the integration between identity management (DIDs) and mutual credit systems.

## Overview

The example demonstrates:

1. **Decentralized Identity (DID) Management**:
   - Creation of DIDs for cooperatives
   - Resolution of DIDs to retrieve DID documents
   - Verification of DID ownership

2. **Mutual Credit System**:
   - Account creation linked to DIDs
   - Credit limit enforcement
   - Transaction execution between accounts
   - Balance tracking

## Key Components

### Identity System

- `DidDocument`: Represents a DID document containing verification methods and services
- `DidManager`: Manages the creation and resolution of DIDs

### Mutual Credit System

- `Account`: Represents a cooperative's account with balance and credit limit
- `Transaction`: Represents a credit transaction between accounts
- `MutualCreditSystem`: Manages accounts and transactions, enforcing credit limits

## Running the Example

```bash
cargo run
```

## Example Output

The example demonstrates:

1. Creating DIDs for two cooperatives
2. Creating mutual credit accounts linked to those DIDs
3. Performing a credit transaction between the accounts
4. Checking account balances
5. Attempting to exceed a credit limit (which fails as expected)

## Real-World Applications

In a real-world implementation, this example would be extended with:

- Cryptographic verification of DIDs
- Federation of DIDs across different networks
- More sophisticated credit limit policies
- Governance mechanisms for credit issuance
- Integration with other economic components

## Relation to ICN Architecture

This example demonstrates a simplified version of two core components of the ICN architecture:

1. **Identity Layer**: Provides the foundation for identifying cooperatives in the network
2. **Economic Layer**: Enables economic interactions between cooperatives through mutual credit

These components work together to create a decentralized network where cooperatives can establish trusted economic relationships. 