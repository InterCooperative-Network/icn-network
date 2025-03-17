# Secure Storage System

The ICN Network provides a robust and secure storage system designed for distributed environments. This documentation covers the architecture, features, and usage of the secure storage system.

## Overview

The storage system is built with security, privacy, and federation in mind, offering:

- **Multi-Federation Storage**: Isolated storage environments with independent encryption
- **End-to-End Encryption**: Multiple encryption algorithms for different security needs
- **Versioning**: Automatic versioning of stored files with secure metadata
- **Key Management**: Secure key storage, sharing, and rotation mechanisms
- **Access Control**: Fine-grained control over who can access stored data
- **Recipient-Specific Encryption**: Public key encryption for specific recipients

## Architecture

The secure storage architecture consists of the following components:

```
┌───────────────────────────────────────┐
│             Storage CLI               │
└───────────────┬───────────────────────┘
                │
┌───────────────▼───────────────────────┐
│           StorageService              │
├───────────────────┬───────────────────┤
│   CryptoService   │  Version Manager  │
├───────────────────┼───────────────────┤
│  Federation Mgr   │   Access Control  │
├───────────────────┴───────────────────┤
│        Storage System Interface       │
└───────────────────────────────────────┘
```

### Components

- **StorageService**: Manages the overall storage operations and coordinates between other components
- **CryptoService**: Handles encryption, decryption, and key management
- **Version Manager**: Tracks file versions and metadata
- **Federation Manager**: Maintains separate storage environments for different federations
- **Access Control**: Enforces permissions and access policies
- **Storage System Interface**: Abstracts the underlying storage mechanism (file system, database, etc.)

## Federation-Based Storage

Each federation in the ICN Network can have its own isolated storage environment:

- **Independent Encryption**: Each federation has its own encryption keys
- **Isolation**: Data in one federation is completely isolated from other federations
- **Customizable Settings**: Each federation can have its own storage policies

## Encryption Methods

The storage system supports multiple encryption methods:

### Symmetric Encryption

- **ChaCha20Poly1305**: Fast, secure symmetric encryption with authentication
- **AES-256-GCM**: Industry-standard symmetric encryption with hardware acceleration
- **Password-Based**: Derives encryption keys from passwords using Argon2

### Asymmetric Encryption

- **X25519**: Secure elliptic curve-based asymmetric encryption
- **Hybrid Encryption**: Combines asymmetric and symmetric encryption for efficiency

## Key Management

The system provides comprehensive key management features:

### Key Types

- **Symmetric Keys**: Used for federation-wide encryption
- **Asymmetric Key Pairs**: Used for recipient-specific encryption
- **Password-Derived Keys**: Generated from user passwords

### Key Operations

- **Generation**: Secure random key generation
- **Storage**: Secure storage of keys with memory protection
- **Rotation**: Support for key rotation and version tracking
- **Import/Export**: Secure key sharing between authorized parties

## Versioning System

All files stored in the system are automatically versioned:

- **Version History**: Complete history of file changes
- **Content Hashing**: Cryptographic verification of file integrity
- **Metadata Tracking**: Records modification times, sizes, and other metadata
- **Version Retrieval**: Ability to retrieve specific versions of a file

## CLI Usage

The secure storage system can be used through the ICN CLI with the following commands:

### Basic Storage Operations

```bash
# Initialize storage with encryption
icn-cli storage init --path ./data --encrypted

# Store a file with encryption
icn-cli storage put --file document.pdf --encrypted --federation finance

# Retrieve a file
icn-cli storage get --key document.pdf --output ./retrieved.pdf --federation finance

# List files in a federation
icn-cli storage list --federation finance

# View version history
icn-cli storage history --key document.pdf --federation finance
```

### Key Management

```bash
# Generate federation encryption key
icn-cli storage generate-key --output ./federation.key

# Export federation key for sharing
icn-cli storage export-key --federation finance --output finance_key.json

# Import federation key
icn-cli storage import-key --federation finance --key-file received_key.json
```

### Recipient-Specific Encryption

```bash
# Generate asymmetric key pair
icn-cli storage generate-key-pair --output-dir ./my_keys

# Encrypt file for specific recipients
icn-cli storage encrypt-for --input sensitive.doc --output sensitive.enc --recipients "user1_pub.key,user2_pub.key"

# Decrypt file with private key
icn-cli storage decrypt-with --input sensitive.enc --output decrypted.doc --private-key ./my_keys/private.key
```

## Security Considerations

### Content Integrity

All stored files include:
- SHA-256 content hashing
- Authentication tags from authenticated encryption
- Version metadata tracking

### Key Security

The system protects keys through:
- Secure key generation with cryptographically secure random number generation
- Protected key storage with appropriate file permissions
- Memory protection for keys in use
- Key derivation with Argon2 for password-based encryption

### Access Control

Access to stored files is controlled through:
- Federation-level isolation
- Cryptographic access control (possession of correct keys)
- Recipient-specific encryption for targeted sharing

## Implementation Details

### Encryption Process

1. **Symmetric Encryption**:
   - Generate random nonce/IV
   - Encrypt data with chosen algorithm and federation key
   - Store encryption metadata with file

2. **Asymmetric Encryption**:
   - Generate random symmetric content key
   - Encrypt content with symmetric key
   - Encrypt content key for each recipient using their public key
   - Store recipient information and encrypted keys with ciphertext

### Authenticated Encryption

All encryption operations include:
- **Authentication Tags**: Ensures data integrity and authenticity
- **Content Verification**: Prevents tampering with stored files
- **Metadata Protection**: Secures version and file metadata

## Best Practices

When using the secure storage system:

1. **Federation Management**:
   - Create separate federations for different security classifications
   - Limit cross-federation key sharing

2. **Key Management**:
   - Regularly rotate encryption keys
   - Securely back up federation keys
   - Use hardware security for private key storage when possible

3. **Recipient Encryption**:
   - Verify recipient public keys before encryption
   - Limit the number of recipients per file for security
   - Use a secure channel for initial key exchange

4. **General Security**:
   - Keep the CLI itself updated to the latest version
   - Apply principle of least privilege for access to storage operations
   - Use strong passwords for password-derived keys 