# Identity-Integrated Storage System

The ICN Network has been enhanced with an Identity-Integrated Storage System that combines the strengths of decentralized identity (DID), governance-controlled policies, and secure storage to create a user-centric, secure, and democratically managed data storage solution.

## System Overview

The Identity-Integrated Storage System builds upon our previously implemented secure storage and governance systems by adding a robust identity layer for authentication and access control. This integration achieves several key objectives:

1. **DID-Based Authentication**: Users can authenticate to the storage system using their decentralized identifiers (DIDs) and cryptographic signatures, eliminating reliance on centralized authentication services.

2. **Integration with Governance**: The system leverages the existing governance framework to enforce access control policies while adding identity-specific capabilities.

3. **Key Rotation Support**: Users can update their DID documents and keys while maintaining access to their resources, enhancing security without disruption.

4. **Federation Member Mapping**: DIDs are mapped to federation member IDs for seamless integration with existing governance policies.

5. **Challenge-Response Authentication**: Secure challenge-response mechanisms protect against replay attacks and ensure only authorized DIDs can access protected resources.

## Core Components

### IdentityProvider

A trait that defines the interface for resolving DIDs, verifying signatures, and maintaining DID-to-member mappings:

```rust
pub trait IdentityProvider {
    async fn resolve_did(&self, did: &str) -> Result<Option<DidDocument>>;
    async fn verify_signature(&self, did: &str, message: &[u8], signature: &[u8]) -> Result<DidVerificationStatus>;
    async fn get_member_id_for_did(&self, did: &str) -> Result<Option<String>>;
}
```

### IdentityStorageService

The central service that integrates identity verification with storage operations:

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

This service handles:
- DID authentication and signature verification
- Mapping DIDs to member IDs for policy enforcement
- Integrating with governance-controlled storage for permission checks
- Caching authentication results for performance optimization

### DID Document Structure

A simplified W3C-compliant DID document structure that includes:
- DID identifier
- Controller information
- Verification methods (cryptographic keys)
- Authentication methods
- Service endpoints

## CLI Commands

New CLI commands have been added to support identity-integrated storage operations:

- `icn-cli identity-storage init`: Initialize the identity storage environment
- `icn-cli identity-storage register-did`: Register a DID document for storage access
- `icn-cli identity-storage store-file`: Store a file with DID authentication
- `icn-cli identity-storage get-file`: Retrieve a file with DID authentication
- `icn-cli identity-storage list-files`: List files accessible to a DID
- `icn-cli identity-storage map-did-to-member`: Map a DID to a federation member ID
- `icn-cli identity-storage create-access-policy`: Create an access policy with DID authentication

## Security Features

The Identity-Integrated Storage System includes several security enhancements:

1. **Cryptographic Authentication**: All operations require cryptographic proof of DID control.
2. **Authentication Caching**: DID authentication results are cached with a configurable time-to-live (TTL) for performance optimization.
3. **Challenge-Response Protocol**: Prevents replay attacks by requiring signatures on unique challenges.
4. **Default-Deny Access Control**: Access is denied by default and only granted through explicit policies.
5. **Key Rotation Support**: Users can update their keys while maintaining access to their resources.

## Demo Script

A comprehensive demonstration script (`examples/identity_storage_demo.sh`) has been created to showcase the capabilities of the identity-integrated storage system, including:

- Creating and registering DID documents
- Mapping DIDs to member IDs
- Creating access control policies
- Storing and retrieving files with DID authentication
- Demonstrating policy updates and their effects
- Showcasing key rotation capabilities

## Documentation

Detailed documentation has been added in `docs/storage/identity-integrated-storage.md`, covering:
- System architecture and components
- DID authentication process
- Key component descriptions
- Usage examples
- Security considerations
- Advanced features
- Future extensions

The main README.md has also been updated to reflect the new identity-integrated storage capabilities.

## Future Extensions

Planned extensions for the Identity-Integrated Storage System include:

1. **Verifiable Credentials**: Integrating verifiable credentials for attribute-based access control.
2. **DID Federation**: Supporting cross-federation DID authentication.
3. **Progressive Trust**: Implementing trust mechanisms based on DID interaction history.
4. **Identity Recovery**: Supporting DID recovery mechanisms for lost keys.
5. **Selective Disclosure**: Enabling selective disclosure of identity attributes for access control.

## Conclusion

The Identity-Integrated Storage System represents a significant advancement in the ICN Network's capabilities, providing a secure, user-centric approach to storage access control through the integration of decentralized identity. By combining DIDs, governance policies, and secure storage, we've created a system that enables fine-grained access control while maintaining user autonomy and privacy. 

# Credential-Based Storage System

The ICN Network has been further enhanced with a Credential-Based Storage System that extends our identity-integrated storage with attribute-based access control through verifiable credentials. This advancement enables more sophisticated and fine-grained authorization based on verified attributes rather than just identity.

## System Overview

The Credential-Based Storage System builds upon our identity-integrated storage by adding support for W3C Verifiable Credentials, allowing access control decisions based on attributes contained within these credentials. This integration achieves several key objectives:

1. **Attribute-Based Access Control**: Access permissions are granted based on verified attributes such as role, department, clearance level, or any other relevant credential data.

2. **Credential Verification**: Cryptographic verification ensures that credentials are authentic, issued by trusted entities, and have not been tampered with.

3. **Freshness Enforcement**: Automatic checking of credential expiration dates and revocation status ensures that access decisions are based on current information.

4. **Fine-Grained Access Rules**: Sophisticated rule matching based on credential types and attribute values enables precise access control policies.

5. **Federation-Governed Trust**: The democratic governance system determines which credential issuers are trusted within the federation.

## Core Components

### VerifiableCredential

A structure representing W3C-compliant verifiable credentials with claims about a subject:

```rust
pub struct VerifiableCredential {
    pub id: String,
    pub types: Vec<String>,
    pub issuer: String,
    pub issuance_date: DateTime<Utc>,
    pub expiration_date: Option<DateTime<Utc>>,
    pub credential_subject: CredentialSubject,
    pub proof: Proof,
    pub revocation_info: Option<RevocationInfo>,
}
```

### CredentialProvider

A trait that defines the interface for resolving and verifying credentials:

```rust
pub trait CredentialProvider {
    async fn resolve_credential(&self, credential_id: &str) -> Result<Option<VerifiableCredential>>;
    async fn verify_credential(&self, credential: &VerifiableCredential) -> Result<CredentialVerificationStatus>;
    async fn is_revoked(&self, credential: &VerifiableCredential) -> Result<bool>;
}
```

### CredentialStorageService

The central service that integrates credential verification with identity-based storage:

```rust
pub struct CredentialStorageService<P: CredentialProvider, I: IdentityProvider> {
    federation: String,
    storage_path: PathBuf,
    credential_provider: P,
    identity_storage: IdentityStorageService<I>,
    access_rules: Vec<CredentialAccessRule>,
}
```

This service handles:
- Credential verification and attribute checking
- Integrating with identity-based storage for DID authentication
- Enforcing access rules based on credential attributes
- Managing access rule persistence

### CredentialAccessRule

A structure defining access rules based on credential types and attributes:

```rust
pub struct CredentialAccessRule {
    pub pattern: String,
    pub credential_types: Vec<String>,
    pub attributes: HashMap<String, String>,
    pub permissions: Vec<Permission>,
}
```

## CLI Commands

New CLI commands have been added to support credential-based storage operations:

- `icn-cli credential-storage init`: Initialize the credential storage environment
- `icn-cli credential-storage register-credential`: Register a verifiable credential
- `icn-cli credential-storage create-access-rule`: Create a credential-based access rule
- `icn-cli credential-storage store-file`: Store a file with credential authentication
- `icn-cli credential-storage get-file`: Retrieve a file with credential authentication
- `icn-cli credential-storage list-files`: List files accessible with a credential
- `icn-cli credential-storage verify-credential`: Verify a specific credential
- `icn-cli credential-storage save-access-rules`: Save credential access rules to a file
- `icn-cli credential-storage load-access-rules`: Load credential access rules from a file

## Security Features

The Credential-Based Storage System includes several security enhancements:

1. **Cryptographic Verification**: All credentials require cryptographic proof of authenticity via signatures.
2. **Expiration Enforcement**: Credentials with expired dates are automatically rejected.
3. **Revocation Checking**: The system checks if credentials have been revoked before granting access.
4. **Attribute Matching**: Fine-grained matching of credential attributes against access rules.
5. **Trust Framework**: Federation-governed decisions on which credential issuers to trust.

## Demo Script

A comprehensive demonstration script (`examples/credential_storage_demo.sh`) has been created to showcase the capabilities of the credential-based storage system, including:

- Creating and registering verifiable credentials with different attributes
- Mapping DIDs to federation member IDs
- Creating access rules based on department and clearance level
- Storing and retrieving files with credential-based authentication
- Demonstrating cross-department access using higher-level credentials
- Showcasing expired credential handling and access denials

## Documentation

Detailed documentation has been added in `docs/storage/credential-based-storage.md`, covering:
- System architecture and components
- Credential verification process
- Access rule structure and matching
- Integration with governance and identity systems
- Security considerations and best practices
- Usage examples and CLI reference
- Future extensions

The main README.md has also been updated to reflect the new credential-based storage capabilities.

## Future Extensions

Planned extensions for the Credential-Based Storage System include:

1. **Selective Disclosure**: Support for Zero-Knowledge Proofs to reveal only necessary attributes.
2. **Delegation Credentials**: Enable temporary delegation of access through special credentials.
3. **Credential Schemas**: Define and validate credential schemas for structure consistency.
4. **Federation-to-Federation Trust**: Cross-federation credential acceptance frameworks.
5. **Automated Credential Renewal**: Workflows for updating and renewing credentials.

## Conclusion

The Credential-Based Storage System represents a significant advancement in the ICN Network's authorization capabilities, enabling true attribute-based access control that goes beyond simple identity verification. By leveraging verifiable credentials, we provide a powerful mechanism for implementing sophisticated security policies that match real-world organizational structures and roles. This enhancement further strengthens the ICN Network's position as a comprehensive solution for secure, decentralized data management. 

## ICN Network Distributed Compute System

The ICN Network Distributed Compute System represents a significant advancement in distributed computing by seamlessly integrating with our identity and credential frameworks for secure, democratically governed computation.

### System Overview

The Distributed Compute System allows federation members to:

- **Execute computations** on data stored within the secure storage system
- **Maintain security context** throughout the entire data lifecycle
- **Apply governance policies** to compute resources and operations
- **Enforce credential verification** for all compute operations
- **Track provenance** of data and computational results

By building on top of our existing identity-integrated and credential-based storage systems, the Distributed Compute System inherits all security properties and governance controls, ensuring that compute operations are subject to the same democratically determined policies as data storage.

### Core Components

The system consists of several key components:

1. **ComputeJob Structure**: Represents a computational task with clear specifications:

```rust
struct ComputeJob {
    id: String,
    user_did: String,
    name: String,
    command: String,
    args: Vec<String>,
    resource_requirements: ResourceRequirements,
    input_files: HashMap<String, String>,
    output_files: HashMap<String, String>,
    status: ComputeJobStatus,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}
```

2. **ResourceRequirements Structure**: Defines the computational resources needed:

```rust
struct ResourceRequirements {
    cpu_cores: u32,
    memory_mb: u32,
    gpu_memory_mb: Option<u32>,
}
```

3. **ComputeStorageService**: Integrates compute operations with secure storage:

```rust
struct ComputeStorageService<I: IdentityProvider, C: CredentialProvider> {
    workspace_dir: PathBuf,
    federation_name: String,
    identity_storage: IdentityStorageService<I>,
    credential_storage: CredentialStorageService<I, C>,
    jobs: RwLock<HashMap<String, ComputeJob>>,
}
```

4. **Job Execution Environment**: Provides isolated execution environments for compute tasks, preventing interference between different users' jobs.

### CLI Commands

The system provides comprehensive CLI commands for managing compute operations:

- **init**: Initializes the compute environment
  ```
  icn-cli compute init --workspace ./compute_workspace --federation demo-fed
  ```

- **submit-job**: Submits a raw compute job with fine-grained control
  ```
  icn-cli compute submit-job --did "DID" --challenge "CHALLENGE" --signature "SIG" 
    --credential-id "CRED_ID" --name "JOB_NAME" --command "CMD" --args "ARG1,ARG2" 
    --cpu 2 --memory 1024 --input-files "SRC:DEST" --output-files "SRC:DEST"
  ```

- **process-data**: Higher-level command for data processing workloads
  ```
  icn-cli compute process-data --did "DID" --credential-id "CRED_ID" 
    --name "JOB_NAME" --command "python3" --args "script.py,input.csv,output.txt" 
    --input-files "input.csv:input.csv,script.py:script.py" 
    --output-files "output.txt:results/output.txt"
  ```

- **get-job-status/get-job/list-jobs**: Commands for job monitoring and management
- **cancel-job**: Cancels a running job
- **get-job-logs**: Retrieves execution logs
- **upload-job-outputs**: Uploads job results to secure storage

### Security Features

The Distributed Compute System incorporates several security mechanisms:

1. **DID-based Authentication**: All compute operations require valid DIDs with proper signatures
2. **Credential Verification**: Compute access requires appropriate verifiable credentials
3. **Isolated Execution**: Jobs run in isolated environments to prevent interference
4. **Governance Integration**: Compute resources are subject to democratic governance
5. **Secure Data Transfer**: All data movement between storage and compute environments maintains security context
6. **Audit Logging**: Comprehensive logging of all compute operations for accountability

### Demo Script

A demonstration script (`examples/compute_demo.sh`) showcases the system's capabilities, including:

1. Setting up the compute environment
2. Creating and registering DID documents
3. Issuing and verifying compute credentials
4. Storing input data and processing scripts
5. Submitting compute jobs
6. Monitoring job execution
7. Retrieving and verifying results

### Future Extensions

1. **Distributed Compute Scheduling**: Efficiently distribute jobs across multiple compute nodes
2. **Federated Learning Support**: Enable privacy-preserving machine learning across federations
3. **Zero-Knowledge Compute**: Integrate zero-knowledge proofs for verified computation
4. **Compute Marketplace**: Allow federations to share and trade compute resources
5. **Real-time Collaboration**: Enable collaborative compute sessions for interactive data analysis

### Conclusion

The ICN Network Distributed Compute System represents a significant advancement in secure, democratically governed distributed computing. By integrating with our identity and credential frameworks, it ensures that computation maintains the same security properties and governance controls as data storage, creating a comprehensive solution for secure, collaborative data processing in a federated environment. 

## Storage

- [Storage CLI Commands](storage_cli.md)
- [Distributed Storage](distributed_storage.md)
- [Storage Federation](storage/README.md)
  - [Encrypted Storage](storage/encrypted.md)
  - [Federation Storage](storage/federation.md)
  - [Governance Storage](storage/governance.md)
  - [Identity Storage](storage/identity.md)
  - [Credential Storage](storage/credential.md)

## Domain-Specific Language (DSL)

- [DSL Overview](dsl/README.md) 