# Governance-Controlled Storage System

The ICN Network provides a governance-controlled storage system that integrates secure storage with democratic governance. This enables federations to collectively manage storage resources, enforce access controls, and set storage policies through democratic decision-making.

## Overview

The governance-controlled storage system combines two core components of the ICN Network:

1. **Secure Encrypted Storage**: The existing storage system with encryption, versioning, and federation support
2. **Democratic Governance**: The proposal and voting system for collective decision-making

By integrating these components, federations can:

- Set storage quotas for the federation and individual members
- Define access control policies determining who can access specific files
- Control encryption requirements and algorithms
- Manage data retention and replication policies
- All through democratic processes with voting by federation members

## Architecture

The governance-controlled storage architecture consists of the following components:

```
┌───────────────────────────────────────┐
│             CLI Interface             │
└───────────────┬───────────────────────┘
                │
┌───────────────▼───────────────────────┐
│     GovernanceStorageService          │
├───────────────────┬───────────────────┤
│   StorageService  │ GovernanceService │
├───────────────────┼───────────────────┤
│   Policy Manager  │   Access Control  │
├───────────────────┴───────────────────┤
│        Storage System Interface       │
└───────────────────────────────────────┘
```

### Components

- **GovernanceStorageService**: The main integration layer combining storage and governance
- **StorageService**: Provides encrypted, versioned storage capabilities
- **GovernanceService**: Enables democratic proposal creation and voting
- **Policy Manager**: Manages and enforces storage policies
- **Access Control**: Enforces permissions based on access control policies

## Storage Policies

The system supports several types of storage policies that can be proposed, voted on, and enforced:

### Federation Quota Policy

Sets overall storage limits for the entire federation:

```json
{
  "target_id": "federation",
  "max_bytes": 10485760,
  "max_files": 100,
  "max_file_size": 1048576
}
```

### Member Quota Policy

Sets storage limits for specific members:

```json
[
  {
    "target_id": "member1@example.org",
    "max_bytes": 1048576,
    "max_files": 10,
    "max_file_size": 524288
  },
  {
    "target_id": "member2@example.org",
    "max_bytes": 2097152,
    "max_files": 20,
    "max_file_size": 1048576
  }
]
```

### Access Control Policy

Defines who can access which files using path patterns:

```json
[
  {
    "member_id": "admin@example.org",
    "path_pattern": "*",
    "can_read": true,
    "can_write": true,
    "can_grant": true
  },
  {
    "member_id": "alice@example.org",
    "path_pattern": "public*",
    "can_read": true,
    "can_write": true,
    "can_grant": false
  },
  {
    "member_id": "bob@example.org",
    "path_pattern": "bob*",
    "can_read": true,
    "can_write": true,
    "can_grant": false
  }
]
```

### Retention Policy

Controls how long data is retained and how many versions are kept:

```json
[
  {
    "path_pattern": "temp*",
    "max_age_seconds": 604800,
    "min_versions": 1,
    "max_versions": 3
  },
  {
    "path_pattern": "important*",
    "min_versions": 5,
    "max_versions": 10
  }
]
```

### Encryption Algorithms Policy

Specifies which encryption algorithms are allowed:

```json
{
  "allowed_algorithms": ["ChaCha20Poly1305", "Aes256Gcm", "X25519"],
  "required_for_patterns": ["confidential*", "sensitive*"],
  "default_algorithm": "Aes256Gcm"
}
```

### Replication Policy

Defines how data is replicated across storage nodes:

```json
{
  "default_replicas": 3,
  "min_replicas": 2,
  "patterns": [
    {"path_pattern": "critical*", "replicas": 5},
    {"path_pattern": "temp*", "replicas": 1}
  ]
}
```

## Policy Enforcement

The system enforces policies through several mechanisms:

1. **Access Control Enforcement**: Checks permissions before allowing read/write operations
2. **Quota Enforcement**: Prevents storage that would exceed quotas
3. **Encryption Enforcement**: Ensures required encryption is applied
4. **Retention Enforcement**: Manages version history according to policy

## CLI Usage

### Storage Operations with Governance Checks

```bash
# Store a file with governance permission checks
icn-cli governed-storage store-file --file document.pdf --member alice@example.org

# Retrieve a file with governance permission checks
icn-cli governed-storage get-file --key document.pdf --member alice@example.org

# List files accessible to a member
icn-cli governed-storage list-files --member alice@example.org
```

### Managing Storage Policies

```bash
# Propose a new storage policy
icn-cli governed-storage propose-policy \
  --proposer alice@example.org \
  --title "New Member Quotas" \
  --description "Updated storage quotas for members" \
  --policy-type member-quota \
  --content-file quotas.json

# List active storage policies
icn-cli governed-storage list-policies

# Show JSON schema for a policy type
icn-cli governed-storage show-schema --policy-type federation-quota

# Apply an approved policy
icn-cli governed-storage apply-policy --proposal-id 12345abcde
```

## Democratic Governance Process

Implementing a new storage policy follows this process:

1. **Policy Proposal**: A member proposes a new policy (e.g., access control rules)
2. **Deliberation**: Members discuss the proposal and its implications
3. **Voting**: Members vote on the proposal
4. **Execution**: If approved, the policy is applied to the storage system

This democratic process ensures that storage management reflects the collective will of the federation rather than being controlled by a central authority.

## Benefits

The governance-controlled storage system provides several benefits:

- **Democratic Control**: Storage policies reflect collective decisions
- **Fine-grained Access Control**: Precise control over who can access what
- **Resource Management**: Prevent any member from consuming excessive resources
- **Policy Transparency**: Clear, visible policies with audit trails
- **Flexible Adaptation**: Policies can evolve through democratic processes

## Use Cases

### Multi-Stakeholder Data Management

Organizations with multiple stakeholders can use governance-controlled storage to ensure fair access and resource allocation, with policies determined through democratic processes.

### Sensitive Information Handling

For federations that handle sensitive information, governance can enforce strict access controls and encryption requirements, with collective oversight through the proposal and voting system.

### Resource-Constrained Environments

When storage resources are limited, governance mechanisms ensure fair allocation through democratically established quotas and priorities.

## Integration with Other Systems

The governance-controlled storage system integrates with other ICN Network components:

- **Identity System**: Uses decentralized identities for authentication
- **Economic System**: Can integrate with resource accounting and compensation
- **Network System**: Works with distributed storage across multiple nodes
- **Application Layer**: Provides governed storage APIs for applications

## Implementation Details

### Policy Storage

Policies are stored in a dedicated area within the federation's storage, with each policy saved as a JSON file with metadata about its creation, approval, and status.

### Permission Checking

The system implements efficient permission checking using pattern matching against the path patterns defined in access control policies.

### Quota Tracking

The storage system tracks usage at both the member and federation levels to enforce quota policies.

## Future Extensions

- **Delegation**: Allow members to delegate specific access rights to others
- **Conditional Policies**: Policies that adapt based on external conditions
- **Policy Analytics**: Tools to analyze policy effectiveness and impact
- **Multi-Federation Policies**: Coordinated policies across multiple federations 