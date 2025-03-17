# Credential-Based Storage in ICN Network

## Overview

The Credential-Based Storage system in ICN Network extends our identity-integrated storage with advanced attribute-based access control through verifiable credentials. This system combines the security of our encrypted storage, the democratic governance of our federation framework, the authentication of our DID-based identity system, and now adds fine-grained authorization based on verified attributes.

Verifiable Credentials (VCs) are cryptographically verifiable claims issued by trusted entities. They contain statements about an entity (subject) that can be independently verified. The ICN Network leverages this technology to enable more sophisticated access control policies beyond simple identity verification.

## Key Features

- **Attribute-Based Access Control**: Grant permissions based on verified attributes like role, department, or clearance level
- **Credential Verification**: Cryptographically verify credentials before granting access
- **Credential Revocation Checking**: Check if credentials have been revoked before authorizing access
- **Expiration Enforcement**: Automatically deny access when credentials have expired
- **Fine-Grained Access Rules**: Create sophisticated rules matching credential types and attribute values
- **Rule Persistency**: Save and load access rules for consistent policy enforcement
- **Integration with Governance**: Credential policies can be managed through federation governance

## Architecture

The credential-based storage system is built as a layer on top of our identity-integrated storage system:

```
┌────────────────────────────────┐
│    Credential-Based Storage    │
├────────────────────────────────┤
│     Identity-Integrated        │
│          Storage               │
├────────────────────────────────┤
│    Governance-Controlled       │
│          Storage               │
├────────────────────────────────┤
│      Encrypted Storage         │
└────────────────────────────────┘
```

### Core Components

1. **VerifiableCredential**: Represents a W3C-compliant verifiable credential with claims about a subject
2. **CredentialProvider**: Interface for resolving and verifying credentials
3. **CredentialAccessRule**: Defines access rules based on credential types and attributes
4. **CredentialStorageService**: Main service orchestrating credential-based storage operations

## Credential Structure

Verifiable credentials in ICN follow the W3C standard format:

```json
{
  "@context": [
    "https://www.w3.org/2018/credentials/v1",
    "https://www.w3.org/2018/credentials/examples/v1"
  ],
  "id": "credential:1",
  "type": ["VerifiableCredential", "DepartmentCredential"],
  "issuer": "did:icn:issuer",
  "issuanceDate": "2023-01-01T00:00:00Z", 
  "expirationDate": "2023-12-31T23:59:59Z",
  "credentialSubject": {
    "id": "did:icn:subject",
    "department": "Engineering",
    "role": "Developer"
  },
  "proof": {
    "type": "Ed25519Signature2020",
    "created": "2023-01-01T00:00:00Z",
    "verificationMethod": "did:icn:issuer#key-1",
    "proofPurpose": "assertionMethod",
    "jws": "eyJhb...5fQ"
  }
}
```

## Access Rule Structure

Access rules define what credentials and attributes are required to access specific files:

```rust
pub struct CredentialAccessRule {
    /// Pattern to match against file paths
    pub pattern: String,
    
    /// Required credential types (ANY match)
    pub credential_types: Vec<String>,
    
    /// Required attributes in the credential (ALL must match)
    pub attributes: HashMap<String, String>,
    
    /// Permissions granted if this rule matches
    pub permissions: Vec<Permission>,
}
```

A rule matches when:
1. The file path matches the rule's pattern
2. The presented credential has ANY of the required credential types
3. The credential contains ALL of the required attributes with matching values

## Usage Examples

### Initialize Credential Storage

```bash
icn-cli credential-storage init \
  --path ./storage \
  --federation my-federation
```

### Register a Credential

```bash
icn-cli credential-storage register-credential \
  --credential credential.json \
  --federation my-federation
```

### Create a Credential-Based Access Rule

```bash
icn-cli credential-storage create-access-rule \
  --did "did:icn:alice" \
  --challenge "timestamp=1621500000" \
  --signature "alice_signature" \
  --pattern "hr_*" \
  --credential-types "DepartmentCredential" \
  --attributes '{"department": "HR"}' \
  --permissions "read,write" \
  --federation my-federation
```

### Store a File with Credential Authentication

```bash
icn-cli credential-storage store-file \
  --did "did:icn:alice" \
  --challenge "timestamp=1621500010" \
  --signature "alice_signature" \
  --credential-id "credential:1" \
  --file document.txt \
  --key "document.txt" \
  --encrypted \
  --federation my-federation
```

### Retrieve a File with Credential Authentication

```bash
icn-cli credential-storage get-file \
  --did "did:icn:alice" \
  --challenge "timestamp=1621500030" \
  --signature "alice_signature" \
  --credential-id "credential:1" \
  --key "document.txt" \
  --output "retrieved_document.txt" \
  --federation my-federation
```

## Security Considerations

### Trust Framework

The security of the credential-based system depends on:

1. **Issuer Trust**: Federations must decide which credential issuers to trust
2. **Credential Integrity**: Verifying cryptographic proofs to ensure credentials are authentic
3. **Freshness Checking**: Ensuring credentials haven't expired or been revoked
4. **Secure Transport**: All operations should use encrypted connections

### Best Practices

- Use specific credential types rather than accepting all types
- Define granular access patterns to limit the scope of each rule
- Regularly rotate credentials and check revocation status
- Implement least-privilege principles by carefully limiting attributes

## Integration with Governance

Federations can democratically manage credential-based storage through governance:

1. **Issuer Policies**: Vote on which issuers are trusted in the federation
2. **System-Wide Rules**: Create baseline access rules through governance proposals
3. **Credential Type Standards**: Define standard credential types and attributes for the federation
4. **Role-Based Templates**: Create standard access patterns for common roles

## Demonstration

The `examples/credential_storage_demo.sh` script provides a complete demonstration of the credential-based storage system. It showcases:

1. Creating and registering DIDs for different users
2. Issuing various types of credentials with different attributes
3. Creating access rules based on department and clearance level
4. Storing and retrieving files with credential-based authorization
5. Handling expired credentials and demonstrating access failures

## Command Reference

| Command | Description |
|---------|-------------|
| `init` | Initialize credential storage environment |
| `register-credential` | Register a verifiable credential |
| `create-access-rule` | Create a credential-based access rule |
| `store-file` | Store a file with credential authentication |
| `get-file` | Retrieve a file with credential authentication |
| `list-files` | List files accessible with a credential |
| `verify-credential` | Verify a specific credential |
| `save-access-rules` | Save credential access rules to a file |
| `load-access-rules` | Load credential access rules from a file |

## Future Extensions

1. **Selective Disclosure**: Support for Zero-Knowledge Proofs to reveal only necessary attributes
2. **Delegation Credentials**: Enable temporary delegation of access through special credentials
3. **Credential Schemas**: Define and validate credential schemas for structure consistency
4. **Federation-to-Federation Trust**: Cross-federation credential acceptance frameworks
5. **Automated Credential Renewal**: Workflows for updating and renewing credentials

## Conclusion

The Credential-Based Storage system represents a significant advancement in ICN Network's security model. By combining verifiable credentials with our existing security layers, we enable true attribute-based access control that goes beyond simple identity verification. This approach allows for more nuanced security policies that match real-world organizational structures and roles. 