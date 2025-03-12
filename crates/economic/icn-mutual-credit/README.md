# ICN Mutual Credit System

A mutual credit system implementation for the Intercooperative Network (ICN). This crate provides the core functionality for a mutual credit system, including account management, credit lines, transactions, and credit clearing.

## Features

- **Account Management**: Create and manage accounts with unique DIDs.
- **Credit Lines**: Establish credit lines between accounts with customizable terms.
- **Transactions**: Process direct transfers and path-based transfers.
- **Credit Clearing**: Implement circular credit clearing to optimize credit utilization.
- **Confidential Transactions**: Support for privacy-preserving transactions using cryptographic commitments.

## Confidential Transactions

The mutual credit system supports confidential transactions, which allow users to transact without revealing the transaction amount to third parties. This is implemented using Pedersen commitments and range proofs.

### How Confidential Transactions Work

1. **Pedersen Commitments**: A Pedersen commitment is a cryptographic primitive that allows a user to commit to a value without revealing it. The commitment is binding (the user cannot change the value later) and hiding (the value is not revealed).

2. **Range Proofs**: A range proof is a zero-knowledge proof that proves that a committed value lies within a specific range, without revealing the value itself. This is used to ensure that transaction amounts are positive and within valid bounds.

3. **Transaction Flow**:
   - The sender creates a transaction with a Pedersen commitment to the amount.
   - The sender generates a range proof to prove that the amount is valid.
   - The transaction is processed normally, but the amount is hidden from third parties.
   - Only the sender and recipient can reveal the actual amount using the blinding factor.

### Usage Example

```rust
// Create a confidential transaction
let confidential_tx_id = processor.create_confidential_transaction(
    &sender_did,
    &recipient_did,
    Amount::new(150),
    Some("Confidential payment".to_string()),
).await?;

// Process the transaction
processor.process_pending_transactions().await;

// Only the sender and recipient can reveal the amount
// (In a real implementation, this would require secure key exchange)
#[cfg(test)]
let revealed_amount = processor.reveal_confidential_amount(&confidential_tx_id)?;
```

For a detailed explanation of the confidential transactions system, see [CONFIDENTIAL_TRANSACTIONS.md](./CONFIDENTIAL_TRANSACTIONS.md).

## Implementation Details

The confidential transactions implementation includes:

- `PedersenCommitment`: A cryptographic commitment to an amount.
- `RangeProof`: A zero-knowledge proof that the committed amount is within a valid range.
- `BlindingFactor`: A random value used to blind the amount in the commitment.
- `ConfidentialTransaction`: A transaction with a hidden amount.
- `ConfidentialTransactionProcessor`: A processor for creating and verifying confidential transactions.

## Security Considerations

In a production environment, the following security considerations should be addressed:

1. **Secure Blinding Factor Exchange**: The blinding factor must be securely shared between the sender and recipient.
2. **Strong Cryptographic Primitives**: Use well-vetted cryptographic libraries for commitments and proofs.
3. **Proper Range Validation**: Ensure that range proofs are properly validated to prevent negative amounts.
4. **Privacy Leakage**: Be aware that transaction patterns may still leak information even if amounts are hidden.

## Future Enhancements

- **Confidential Credit Lines**: Hide credit line limits and balances.
- **Anonymous Transactions**: Hide sender and recipient identities.
- **Confidential Credit Clearing**: Perform credit clearing without revealing individual transaction amounts.
- **Threshold Cryptography**: Split blinding factors among multiple parties for enhanced security.

## Running the Examples

The mutual credit system includes several examples that demonstrate its functionality:

### Basic Transfer Example
```bash
cargo run --example basic_transfer
```
This example demonstrates basic account creation and direct transfers between accounts.

### Credit Clearing Example
```bash
cargo run --example credit_clearing
```
This example shows how multilateral credit clearing works to reduce circular debt in a network.

### Confidential Transactions Examples

#### Simple Confidential Transaction
```bash
cargo run --example confidential_tx
```
This example demonstrates a basic confidential transaction between two parties (Alice and Bob), showing how transaction amounts can be hidden while still maintaining the integrity of the system.

#### Confidential Credit Chain
```bash
cargo run --example confidential_credit_chain
```
This more complex example shows how confidential transactions can be used in a supply chain scenario with multiple participants (Producer, Manufacturer, Distributor, and Retailer), protecting sensitive pricing information while enabling the flow of value through the network.

## Implementation Status

The confidential transactions feature is now fully implemented and tested. The implementation includes:

1. Pedersen commitments for hiding transaction amounts
2. Range proofs to validate that committed amounts are within acceptable bounds
3. A confidential transaction processor that integrates with the existing transaction system
4. Blinding factors for secure amount revelation to authorized parties
5. Example code demonstrating the functionality in various scenarios

All tests are passing, and the examples run successfully, demonstrating that the confidential transactions feature is ready for use in the mutual credit system.

## License

This project is licensed under the MIT License - see the LICENSE file for details. 