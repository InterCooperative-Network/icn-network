# Identity-Integrated Storage System

The ICN Network's Identity-Integrated Storage System combines the strengths of secure storage, governance-controlled policies, and decentralized identity to provide a robust, secure, and user-centric data storage solution.

## Overview

The Identity-Integrated Storage System extends the capabilities of the Governance-Controlled Storage by integrating decentralized identity (DID) for fine-grained access control and authentication. This integration allows:

1. **DID-Based Authentication**: Users authenticate using their DIDs and cryptographic signatures.
2. **Identity-to-Member Mapping**: DIDs are mapped to federation member IDs for policy enforcement.
3. **Key Rotation Support**: Users can update their DID documents and keys while maintaining access to resources.
4. **Governance Policy Integration**: Storage policies are enforced based on DID authentication and member mappings.

## Architecture

The Identity-Integrated Storage System is built on top of the following components:

1. **Identity Provider**: Resolves DIDs, verifies signatures, and maintains DID-to-member mappings.
2. **Governance Storage Service**: Enforces access control policies and manages storage resources.
3. **Identity Storage Service**: Integrates identity verification with storage operations.

```
┌─────────────────┐     ┌───────────────────┐     ┌───────────────────┐
│                 │     │                   │     │                   │
│  Storage System │◄────┤ Governance System │◄────┤  Identity System  │
│                 │     │                   │     │                   │
└────────┬────────┘     └─────────┬─────────┘     └─────────┬─────────┘
         │                        │                         │
         │                        │                         │
         ▼                        ▼                         ▼
┌─────────────────┐     ┌───────────────────┐     ┌───────────────────┐
│                 │     │                   │     │                   │
│ Storage Service │◄────┤ Governance Storage│◄────┤ Identity Storage  │
│                 │     │     Service       │     │     Service       │
└─────────────────┘     └───────────────────┘     └───────────────────┘
                                                           │
                                                           │
                                                           ▼
                                                  ┌───────────────────┐
                                                  │                   │
                                                  │ Identity Provider │
                                                  │                   │
                                                  └───────────────────┘
```

## DID Authentication Process

The system follows this process for DID authentication:

1. **Challenge Generation**: A unique challenge (typically a timestamp or nonce) is created.
2. **Signature Creation**: The user signs the challenge with their private key.
3. **DID Resolution**: The system resolves the user's DID to retrieve their DID document.
4. **Signature Verification**: The system verifies the signature using the public key from the DID document.
5. **Member Mapping**: The authenticated DID is mapped to a federation member ID.
6. **Policy Enforcement**: Access to storage resources is granted based on governance policies.

## Key Components

### IdentityProvider Trait

The `IdentityProvider` trait defines the interface for interacting with DIDs:

```rust
pub trait IdentityProvider {
    async fn resolve_did(&self, did: &str) -> Result<Option<DidDocument>>;
    async fn verify_signature(&self, did: &str, message: &[u8], signature: &[u8]) -> Result<DidVerificationStatus>;
    async fn get_member_id_for_did(&self, did: &str) -> Result<Option<String>>;
}
```

### IdentityStorageService

The `IdentityStorageService` integrates identity verification with storage operations:

```rust
pub struct IdentityStorageService<P: IdentityProvider> {
    federation: String,
    storage_path: PathBuf,
    identity_provider: P,
    governance_storage: GovernanceStorageService,
    auth_cache: HashMap<String, (Instant, String)>,
    cache_ttl: u64,
}
```

Key methods include:

- `authenticate_did`: Verifies a DID signature and maps it to a member ID.
- `store_file`: Stores a file after DID authentication and governance checks.
- `retrieve_file`: Retrieves a file after DID authentication and governance checks.
- `update_did_access_mapping`: Updates the mapping between DIDs and member IDs.
- `create_did_access_policy`: Creates access control policies using DID authentication.

## DID Document Structure

The system uses a simplified DID document structure:

```rust
pub struct DidDocument {
    pub id: String,
    pub controller: Option<String>,
    pub verification_method: Vec<VerificationMethod>,
    pub authentication: Vec<String>,
    pub service: Vec<ServiceEndpoint>,
}

pub struct VerificationMethod {
    pub id: String,
    pub type_: String,
    pub controller: String,
    pub public_key: String,
}

pub struct ServiceEndpoint {
    pub id: String,
    pub type_: String,
    pub service_endpoint: String,
}
```

## CLI Commands

The ICN CLI provides the following commands for interacting with the Identity-Integrated Storage:

- `icn-cli identity-storage init`: Initialize identity storage environment.
- `icn-cli identity-storage register-did`: Register a new DID document.
- `icn-cli identity-storage store-file`: Store a file with DID authentication.
- `icn-cli identity-storage get-file`: Retrieve a file with DID authentication.
- `icn-cli identity-storage list-files`: List files accessible to a DID.
- `icn-cli identity-storage map-did-to-member`: Create a mapping between a DID and a member ID.
- `icn-cli identity-storage create-access-policy`: Create an access policy with DID authentication.

## Usage Examples

### Registering a DID

```bash
icn-cli identity-storage register-did \
    --did "did:icn:alice" \
    --document alice_did.json \
    --federation my-federation
```

### Mapping a DID to a Member ID

```bash
icn-cli identity-storage map-did-to-member \
    --did "did:icn:alice" \
    --member-id "alice" \
    --federation my-federation
```

### Storing a File with DID Authentication

```bash
icn-cli identity-storage store-file \
    --did "did:icn:alice" \
    --challenge "timestamp=1621500000" \
    --signature "alice_signature" \
    --file secret.txt \
    --key "secret.txt" \
    --encrypted \
    --federation my-federation
```

### Retrieving a File with DID Authentication

```bash
icn-cli identity-storage get-file \
    --did "did:icn:alice" \
    --challenge "timestamp=1621500001" \
    --signature "alice_signature" \
    --key "secret.txt" \
    --output "retrieved_secret.txt" \
    --federation my-federation
```

## Security Considerations

The Identity-Integrated Storage System incorporates several security measures:

1. **Cryptographic Authentication**: All operations require cryptographic proof of DID control.
2. **Governance Policy Enforcement**: Access control is maintained through governance policies.
3. **Authentication Caching**: DID authentication results are cached with configurable TTL for performance.
4. **Key Rotation Support**: Users can update their keys while maintaining access to resources.
5. **Default-Deny Policy**: Access is denied by default and only granted through explicit policies.

## Advanced Features

### Key Rotation

The system supports key rotation by updating DID documents. When a user updates their DID document with a new key:

1. The updated DID document is registered.
2. The existing DID-to-member mapping is preserved.
3. The user can immediately authenticate using their new key.

### Access Policy Proposals

Access policies can be proposed through the governance system:

1. A user authenticates with their DID.
2. The user proposes a new access control policy.
3. The proposal is processed through the governance system.
4. If approved, the policy is enacted and enforced.

## Future Extensions

Planned extensions for the Identity-Integrated Storage System include:

1. **Verifiable Credentials**: Integrating verifiable credentials for attribute-based access control.
2. **DID Federation**: Supporting cross-federation DID authentication.
3. **Progressive Trust**: Implementing progressive trust mechanisms based on DID interaction history.
4. **Identity Recovery**: Supporting DID recovery mechanisms for lost keys.
5. **Selective Disclosure**: Enabling selective disclosure of identity attributes for access control.

## Conclusion

The Identity-Integrated Storage System provides a secure, user-centric approach to storage access control through the integration of decentralized identity. By combining the strengths of DIDs, governance policies, and secure storage, the system enables fine-grained access control while maintaining user autonomy and privacy. 