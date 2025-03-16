# Decentralized Identity (DID) Implementation

This document describes the ICN's implementation of W3C Decentralized Identifiers (DIDs), which forms the foundation of identity and authentication in the network.

## Overview

The ICN implements a custom DID method (`did:icn`) that provides:

1. **Decentralized Authentication**: Users log in using DIDs instead of centralized credentials
2. **Verifiable Credentials**: Attributes and permissions are expressed as verifiable credentials
3. **Self-sovereign Identity**: Users control their own identifiers and keys
4. **Federation Support**: DIDs can be federated across cooperative boundaries

## DID Method Specification

### Method Name

The ICN DID method is identified by the method name `icn`:

```
did:icn:<cooperative-id>:<entity-id>
```

### Method-Specific Identifier

The method-specific identifier follows this format:

```
<cooperative-id>:<entity-id>
```

Where:
- `<cooperative-id>`: Identifies the cooperative (e.g., "coopA")
- `<entity-id>`: Identifies the specific user, node, or service within the cooperative

Examples:
- `did:icn:coopA:userX` (a user in cooperative A)
- `did:icn:coopB:node1` (a node in cooperative B)
- `did:icn:federation1:service3` (a service in federation 1)

## DID Document Structure

### Example DID Document

```json
{
  "@context": [
    "https://www.w3.org/ns/did/v1",
    "https://w3id.org/security/suites/ed25519-2020/v1"
  ],
  "id": "did:icn:coopA:userX",
  "controller": ["did:icn:coopA:admin"],
  "verificationMethod": [
    {
      "id": "did:icn:coopA:userX#keys-1",
      "type": "Ed25519VerificationKey2020",
      "controller": "did:icn:coopA:userX",
      "publicKeyMultibase": "z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK"
    },
    {
      "id": "did:icn:coopA:userX#keys-2",
      "type": "Ed25519VerificationKey2020",
      "controller": "did:icn:coopA:userX",
      "publicKeyMultibase": "z6MkhMkypFkqHkzTMmxcakKLgMQZcPj5E5vYNxwd7jVsV8pJ"
    }
  ],
  "authentication": ["did:icn:coopA:userX#keys-1"],
  "assertionMethod": ["did:icn:coopA:userX#keys-1"],
  "keyAgreement": ["did:icn:coopA:userX#keys-2"],
  "service": [
    {
      "id": "did:icn:coopA:userX#wireguard",
      "type": "WireGuardEndpoint",
      "serviceEndpoint": {
        "publicKey": "kXr4/JVeJD8pXjPRpwVsmlVnW8kD9/rv+AcOIk5su3A=",
        "ipv6Address": "fd00:abcd:1234::1"
      }
    },
    {
      "id": "did:icn:coopA:userX#profile",
      "type": "ICNProfile",
      "serviceEndpoint": "https://profiles.coopA.icn/userX"
    }
  ]
}
```

### Key Components

- **Controller**: The DIDs that can modify this DID Document
- **Verification Methods**: Public keys for various verification purposes
- **Authentication**: Methods that can be used to authenticate as this DID
- **Assertion Method**: Methods that can be used to make assertions on behalf of this DID
- **Key Agreement**: Methods that can be used for encrypted communication
- **Service**: Endpoints associated with this DID

## Implementation Details

### DID Manager

```rust
/// Implementation of the DID manager
pub struct DidManager {
    /// Configuration
    config: DidManagerConfig,
    
    /// DID resolver
    resolver: Arc<DidResolver>,
    
    /// Storage service
    storage: Arc<dyn Storage>,
    
    /// DHT service
    dht: Arc<DhtService>,
    
    /// Blockchain client (optional)
    blockchain: Option<Arc<BlockchainClient>>,
    
    /// Federation client
    federation_client: Arc<dyn FederationClient>,
    
    /// Keypairs by DID
    keypairs: RwLock<HashMap<String, HashMap<String, KeyPair>>>,
}

impl DidManager {
    /// Create a new DID
    pub async fn create_did(&self, options: CreateDidOptions) -> Result<(String, DidDocument)> {
        // Generate a new DID
        let did = self.generate_did(
            &options.cooperative_id, 
            &options.entity_id,
        )?;
        
        // Create a new DID document
        let document = self.create_did_document(&did, &options).await?;
        
        // Store the DID document
        self.store_did_document(&did, &document).await?;
        
        // Store the keypair
        self.store_keypair(&did, &options.keypair).await?;
        
        Ok((did, document))
    }
    
    /// Generate a new DID string
    fn generate_did(&self, cooperative_id: &str, entity_id: &str) -> Result<String> {
        if !self.is_valid_id_component(cooperative_id) || !self.is_valid_id_component(entity_id) {
            return Err(Error::invalid_input("Invalid cooperative ID or entity ID"));
        }
        
        Ok(format!("did:icn:{}:{}", cooperative_id, entity_id))
    }
    
    /// Create a DID document
    async fn create_did_document(&self, did: &str, options: &CreateDidOptions) -> Result<DidDocument> {
        let verification_methods = self.create_verification_methods(did, options).await?;
        
        let mut authentication = Vec::new();
        let mut assertion_method = Vec::new();
        let mut key_agreement = Vec::new();
        
        // Use first verification method for authentication by default
        if !verification_methods.is_empty() {
            authentication.push(VerificationMethodReference::Reference(
                format!("{}#keys-1", did)
            ));
        }
        
        // Add assertion method if requested
        if options.add_assertion_method && !verification_methods.is_empty() {
            assertion_method.push(VerificationMethodReference::Reference(
                format!("{}#keys-1", did)
            ));
        }
        
        // Add key agreement if requested
        if options.add_key_agreement && verification_methods.len() > 1 {
            key_agreement.push(VerificationMethodReference::Reference(
                format!("{}#keys-2", did)
            ));
        }
        
        let document = DidDocument {
            id: did.to_string(),
            controller: options.controllers.clone().unwrap_or_default(),
            verification_method: verification_methods,
            authentication,
            assertion_method,
            key_agreement,
            service: options.services.clone().unwrap_or_default(),
        };
        
        Ok(document)
    }
    
    /// Store a DID document
    async fn store_did_document(&self, did: &str, document: &DidDocument) -> Result<()> {
        // Serialize the document
        let serialized = serde_json::to_vec(document)
            .map_err(|e| Error::serialize(format!("Failed to serialize DID document: {}", e)))?;
        
        // Store in DHT
        self.dht.put(
            format!("did:{}", did).as_bytes().to_vec(),
            serialized.clone()
        ).await?;
        
        // Store in local storage
        self.storage.put(
            &format!("dids/{}", did),
            &serialized
        ).await?;
        
        // Store in blockchain if available
        if let Some(blockchain) = &self.blockchain {
            blockchain.store_did_document(did, document).await?;
        }
        
        Ok(())
    }
    
    /// Store a keypair
    async fn store_keypair(&self, did: &str, keypair: &KeyPair) -> Result<()> {
        // Store keypair in encrypted storage
        let key_id = format!("{}#keys-1", did);
        let encrypted = self.encrypt_keypair(keypair)?;
        
        self.storage.put(
            &format!("keys/{}", key_id),
            &encrypted
        ).await?;
        
        // Add to in-memory cache
        let mut keypairs = self.keypairs.write().await;
        keypairs.entry(did.to_string())
            .or_insert_with(HashMap::new)
            .insert(key_id, keypair.clone());
        
        Ok(())
    }
    
    // Other implementation methods...
}
```

### DID Resolver

```rust
/// Implementation of the DID resolver
pub struct DidResolver {
    /// Configuration
    config: DidResolverConfig,
    
    /// Storage service
    storage: Arc<dyn Storage>,
    
    /// DHT service
    dht: Arc<DhtService>,
    
    /// Blockchain client (optional)
    blockchain: Option<Arc<BlockchainClient>>,
    
    /// Cache of resolved documents
    cache: RwLock<LruCache<String, CachedResolution>>,
}

impl DidResolver {
    /// Resolve a DID to a DID document
    pub async fn resolve(&self, did: &str) -> Result<DidDocument> {
        // Validate the DID
        self.validate_did(did)?;
        
        // Check the cache
        if let Some(cached) = self.check_cache(did).await {
            return Ok(cached.document);
        }
        
        // Try to resolve from local storage
        if let Ok(document) = self.resolve_from_storage(did).await {
            self.update_cache(did, &document).await;
            return Ok(document);
        }
        
        // Try to resolve from DHT
        match self.resolve_from_dht(did).await {
            Ok(document) => {
                // Store in local storage
                let _ = self.storage.put(
                    &format!("dids/{}", did),
                    &serde_json::to_vec(&document).unwrap_or_default()
                ).await;
                
                self.update_cache(did, &document).await;
                return Ok(document);
            }
            Err(e) => {
                // If DHT fails, try blockchain
                if let Some(blockchain) = &self.blockchain {
                    match blockchain.resolve_did(did).await {
                        Ok(document) => {
                            // Store in local storage
                            let _ = self.storage.put(
                                &format!("dids/{}", did),
                                &serde_json::to_vec(&document).unwrap_or_default()
                            ).await;
                            
                            self.update_cache(did, &document).await;
                            return Ok(document);
                        }
                        Err(_) => {
                            // Return the original DHT error
                            return Err(e);
                        }
                    }
                } else {
                    return Err(e);
                }
            }
        }
    }
    
    /// Resolve a DID from storage
    async fn resolve_from_storage(&self, did: &str) -> Result<DidDocument> {
        let data = self.storage.get(&format!("dids/{}", did)).await?;
        
        let document: DidDocument = serde_json::from_slice(&data)
            .map_err(|e| Error::deserialize(format!("Failed to deserialize DID document: {}", e)))?;
        
        Ok(document)
    }
    
    /// Resolve a DID from DHT
    async fn resolve_from_dht(&self, did: &str) -> Result<DidDocument> {
        let data = self.dht.get(format!("did:{}", did).as_bytes().to_vec()).await?;
        
        let document: DidDocument = serde_json::from_slice(&data)
            .map_err(|e| Error::deserialize(format!("Failed to deserialize DID document: {}", e)))?;
        
        Ok(document)
    }
    
    // Other implementation methods...
}
```

## Authentication Flow

The authentication process using DIDs involves:

1. **Challenge-Response Authentication**
2. **Signature Verification**
3. **Access Control Using Verifiable Credentials**

### Authentication Challenge Flow

```
┌───────────┐                              ┌───────────┐
│           │                              │           │
│  Client   │                              │  Service  │
│           │                              │           │
└─────┬─────┘                              └─────┬─────┘
      │                                          │
      │  1. Authentication Request (DID)         │
      │─────────────────────────────────────────>│
      │                                          │
      │  2. Challenge (Nonce)                    │
      │<─────────────────────────────────────────│
      │                                          │
      │  3. Signed Challenge                     │
      │─────────────────────────────────────────>│
      │                                          │
      │  4. [Optional] Credential Request        │
      │<─────────────────────────────────────────│
      │                                          │
      │  5. [Optional] Verifiable Credentials    │
      │─────────────────────────────────────────>│
      │                                          │
      │  6. Authentication Result                │
      │<─────────────────────────────────────────│
      │                                          │
```

### Authentication Implementation

```rust
impl AuthenticationManager {
    /// Begin authentication process
    pub async fn begin_authentication(&self, did: &str) -> Result<Challenge> {
        // Validate the DID
        self.did_resolver.validate_did(did)?;
        
        // Generate a challenge
        let nonce = self.generate_nonce()?;
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        let challenge = Challenge {
            did: did.to_string(),
            nonce,
            timestamp,
            expires_at: timestamp + self.config.challenge_ttl,
        };
        
        // Store the challenge
        self.store_challenge(&challenge).await?;
        
        Ok(challenge)
    }
    
    /// Verify an authentication response
    pub async fn verify_authentication(
        &self,
        response: &AuthenticationResponse
    ) -> Result<AuthenticationResult> {
        // Retrieve the challenge
        let challenge = self.get_challenge(&response.challenge_id).await?;
        
        // Check if challenge has expired
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
            
        if challenge.expires_at < now {
            return Err(Error::expired("Challenge has expired"));
        }
        
        // Resolve the DID
        let did_doc = self.did_resolver.resolve(&challenge.did).await?;
        
        // Find the authentication method
        let auth_method = did_doc.find_authentication_method(&response.key_id)
            .ok_or_else(|| Error::not_found(format!("Authentication method {} not found", response.key_id)))?;
        
        // Verify the signature
        let message = self.create_challenge_message(&challenge);
        let public_key = auth_method.get_public_key()?;
        
        let signature = Signature::from_bytes(&response.signature)?;
        
        if !public_key.verify(message.as_bytes(), &signature)? {
            return Err(Error::unauthorized("Invalid signature"));
        }
        
        // Check credentials if required
        let mut verified_credentials = Vec::new();
        if !response.credentials.is_empty() {
            for credential in &response.credentials {
                match self.verify_credential(credential).await {
                    Ok(_) => {
                        verified_credentials.push(credential.clone());
                    }
                    Err(e) => {
                        warn!("Failed to verify credential: {}", e);
                    }
                }
            }
        }
        
        // Generate authentication token
        let token = self.generate_token(&challenge.did, &verified_credentials).await?;
        
        Ok(AuthenticationResult {
            did: challenge.did,
            authenticated: true,
            token,
            verified_credentials,
        })
    }
    
    // Other implementation methods...
}
```

## Integration with Other Components

### 1. WireGuard Integration

DIDs are integrated with WireGuard through service endpoints in the DID Document:

```json
{
  "service": [
    {
      "id": "did:icn:coopA:node1#wireguard",
      "type": "WireGuardEndpoint",
      "serviceEndpoint": {
        "publicKey": "kXr4/JVeJD8pXjPRpwVsmlVnW8kD9/rv+AcOIk5su3A=",
        "ipv6Address": "fd00:abcd:1234::1",
        "listenPort": 51820
      }
    }
  ]
}
```

### 2. Service Discovery

Services advertise themselves through DIDs:

```rust
impl ServiceRegistry {
    /// Register a service
    pub async fn register_service(&self, service: ServiceRegistration) -> Result<()> {
        // Create or update the service's DID
        let did_result = if service.did.is_empty() {
            // Create a new DID for the service
            self.did_manager.create_did(CreateDidOptions {
                cooperative_id: service.cooperative_id.clone(),
                entity_id: format!("service-{}", generate_id()),
                keypair: KeyPair::generate(KeyType::Ed25519)?,
                services: Some(vec![
                    Service {
                        id: format!("#service-{}", service.name),
                        type_: "ICNService".to_string(),
                        service_endpoint: serde_json::to_value(service.clone())?,
                    }
                ]),
                controllers: Some(vec![service.controller_did.clone()]),
                add_assertion_method: true,
                add_key_agreement: true,
            }).await?
        } else {
            // Update existing DID
            let (did, document) = (service.did.clone(), self.did_resolver.resolve(&service.did).await?);
            
            // Add service endpoint
            let mut updated_document = document.clone();
            updated_document.service.push(Service {
                id: format!("{}#service-{}", did, service.name),
                type_: "ICNService".to_string(),
                service_endpoint: serde_json::to_value(service.clone())?,
            });
            
            // Update the DID document
            self.did_manager.update_did_document(&did, updated_document).await?;
            
            (did, updated_document)
        };
        
        // Register service name in the name resolution system
        let service_name = format!("{}.{}.icn", service.name, service.cooperative_id);
        
        let addresses = service.endpoints.iter()
            .map(|endpoint| NameAddress {
                address_type: endpoint.protocol.clone(),
                address: endpoint.address.clone(),
                port: endpoint.port,
                transport: endpoint.transport.clone(),
                priority: endpoint.priority,
            })
            .collect();
            
        self.name_resolver.register_name(
            &service_name,
            &did_result.0,
            addresses
        ).await?;
        
        Ok(())
    }
}
```

### 3. Federation Authentication

Federation across cooperatives uses DIDs for cross-coop authentication:

```rust
impl FederationManager {
    /// Authenticate a DID from another cooperative
    pub async fn authenticate_federated_did(
        &self,
        did: &str,
        challenge_response: &AuthenticationResponse
    ) -> Result<AuthenticationResult> {
        // Extract cooperative ID from DID
        let did_parts: Vec<&str> = did.split(':').collect();
        if did_parts.len() < 3 || did_parts[0] != "did" || did_parts[1] != "icn" {
            return Err(Error::invalid_input("Invalid DID format"));
        }
        
        let coop_id = did_parts[2];
        
        // Check if we have a federation relationship with this cooperative
        let federation_info = self.get_federation_info(coop_id).await?;
        
        // Resolve the DID using the federation's resolver
        let did_doc = match self.resolver.resolve(did).await {
            Ok(doc) => doc,
            Err(_) => {
                // Try to resolve through federation
                self.federation_client.resolve_did(did, &federation_info).await?
            }
        };
        
        // Verify the authentication response
        // ... (authentication logic similar to regular authentication)
        
        // Apply federation policies
        let token = self.apply_federation_policies(
            did,
            &federation_info,
            &verified_credentials
        ).await?;
        
        Ok(AuthenticationResult {
            did: did.to_string(),
            authenticated: true,
            token,
            verified_credentials,
        })
    }
}
```

## Verifiable Credentials

### Credential Structure

```json
{
  "@context": [
    "https://www.w3.org/2018/credentials/v1",
    "https://w3id.org/security/suites/ed25519-2020/v1",
    "https://identity.foundation/presentation-exchange/submission/v1"
  ],
  "id": "did:icn:coopA:cred-1234",
  "type": ["VerifiableCredential", "ICNMemberCredential"],
  "issuer": "did:icn:coopA:admin",
  "issuanceDate": "2023-03-10T12:00:00Z",
  "expirationDate": "2024-03-10T12:00:00Z",
  "credentialSubject": {
    "id": "did:icn:coopA:userX",
    "role": "member",
    "permissions": ["read", "write", "connect"],
    "memberSince": "2023-01-15T00:00:00Z"
  },
  "proof": {
    "type": "Ed25519Signature2020",
    "created": "2023-03-10T12:00:00Z",
    "verificationMethod": "did:icn:coopA:admin#keys-1",
    "proofPurpose": "assertionMethod",
    "proofValue": "z58DAdFfa9SkqZMVPxAQpic6FPWDBNLHBcuiPFUQDzLQEFzCLRStuEnTAcEDyrNrLLWxYX2ZFHRqH8E7JjSBDKnHK"
  }
}
```

### Credential Operations

```rust
impl CredentialManager {
    /// Issue a credential
    pub async fn issue_credential(
        &self, 
        credential_type: &str,
        subject_did: &str,
        claims: HashMap<String, Value>,
        options: &CredentialOptions
    ) -> Result<VerifiableCredential> {
        // Get the issuer DID
        let issuer_did = options.issuer_did.clone()
            .unwrap_or_else(|| self.config.default_issuer_did.clone());
        
        // Check if issuer has key
        let issuer_keypair = self.get_keypair(&issuer_did).await?;
        
        // Create credential
        let id = format!("did:icn:{}:cred-{}", self.extract_coop_id(&issuer_did)?, generate_id());
        
        let now = chrono::Utc::now();
        let expiration = now + chrono::Duration::days(options.validity_days.unwrap_or(365) as i64);
        
        let mut credential = VerifiableCredential {
            context: vec![
                "https://www.w3.org/2018/credentials/v1".to_string(),
                "https://w3id.org/security/suites/ed25519-2020/v1".to_string()
            ],
            id,
            types: vec!["VerifiableCredential".to_string(), credential_type.to_string()],
            issuer: issuer_did.clone(),
            issuance_date: now.to_rfc3339(),
            expiration_date: Some(expiration.to_rfc3339()),
            credential_subject: CredentialSubject {
                id: subject_did.to_string(),
                claims,
            },
            proof: None,
        };
        
        // Add extension contexts
        if let Some(contexts) = &options.additional_contexts {
            credential.context.extend(contexts.clone());
        }
        
        // Create the proof
        let proof = self.create_proof(&credential, &issuer_keypair, &issuer_did).await?;
        credential.proof = Some(proof);
        
        // Store the credential
        if options.store {
            self.store_credential(&credential).await?;
        }
        
        Ok(credential)
    }
    
    /// Verify a credential
    pub async fn verify_credential(&self, credential: &VerifiableCredential) -> Result<VerificationResult> {
        // Check expiration
        if let Some(expiration) = &credential.expiration_date {
            let expiration_date = chrono::DateTime::parse_from_rfc3339(expiration)
                .map_err(|_| Error::invalid_input("Invalid expiration date format"))?;
                
            if expiration_date < chrono::Utc::now() {
                return Ok(VerificationResult {
                    valid: false,
                    reason: Some("Credential has expired".to_string()),
                });
            }
        }
        
        // Check if proof exists
        let proof = match &credential.proof {
            Some(p) => p,
            None => return Ok(VerificationResult {
                valid: false,
                reason: Some("Credential has no proof".to_string()),
            }),
        };
        
        // Resolve issuer DID
        let issuer_did_doc = self.did_resolver.resolve(&credential.issuer).await?;
        
        // Find verification method
        let verification_method = issuer_did_doc.find_verification_method(&proof.verification_method)
            .ok_or_else(|| Error::not_found(format!("Verification method not found: {}", proof.verification_method)))?;
        
        // Verify the signature
        let public_key = verification_method.get_public_key()?;
        
        // Create a credential copy without the proof for verification
        let mut credential_for_verification = credential.clone();
        credential_for_verification.proof = None;
        
        let message = serde_json::to_string(&credential_for_verification)
            .map_err(|e| Error::serialize(format!("Failed to serialize credential: {}", e)))?;
            
        let signature = decode_signature(&proof.proof_value)?;
        
        let valid = public_key.verify(message.as_bytes(), &signature)?;
        
        Ok(VerificationResult {
            valid,
            reason: if valid { None } else { Some("Invalid signature".to_string()) },
        })
    }
}
```

## Security Considerations

### 1. Key Management

- **Key Generation**: Uses cryptographically secure random number generators
- **Key Storage**: Private keys stored in encrypted form
- **Key Rotation**: Supports rotation of keys without changing the DID
- **Key Recovery**: Optional key recovery mechanisms through trusted controllers

### 2. Trust Model

- **Cooperative Trust Anchors**: Each cooperative has trust anchors
- **Federation Trust**: Cooperatives form trust relationships through federations
- **Credential-based Trust**: Granular trust through verifiable credentials

### 3. Revocation

DIDs and credentials can be revoked through:

- **Revocation Lists**: Published lists of revoked DIDs and credentials
- **Status Service**: Real-time status check service
- **Blockchain Records**: Immutable revocation records on the blockchain

## Client API Examples

### Creating a DID

```rust
// Create a DID manager
let did_manager = DidManager::new(
    storage.clone(),
    dht.clone(),
    Some(blockchain.clone()),
    federation_client.clone(),
    DidManagerConfig::default(),
).await?;

// Generate a keypair
let keypair = KeyPair::generate(KeyType::Ed25519)?;

// Create a DID
let (did, document) = did_manager.create_did(CreateDidOptions {
    cooperative_id: "coopA".to_string(),
    entity_id: "userX".to_string(),
    keypair: keypair.clone(),
    add_assertion_method: true,
    add_key_agreement: true,
    services: Some(vec![
        Service {
            id: "#profile".to_string(),
            type_: "ICNProfile".to_string(),
            service_endpoint: "https://profiles.coopA.icn/userX".to_string(),
        }
    ]),
    controllers: None,
}).await?;

println!("Created DID: {}", did);
```

### Authenticating with a DID

```rust
// Create an authentication manager
let auth_manager = AuthenticationManager::new(
    did_resolver.clone(),
    credential_manager.clone(),
    AuthenticationConfig::default(),
).await?;

// Begin authentication
let challenge = auth_manager.begin_authentication("did:icn:coopA:userX").await?;

// Sign the challenge with the user's keypair
let message = format!("{}:{}:{}", challenge.did, challenge.nonce, challenge.timestamp);
let signature = keypair.sign(message.as_bytes())?;

// Create authentication response
let response = AuthenticationResponse {
    challenge_id: challenge.id.clone(),
    key_id: "did:icn:coopA:userX#keys-1".to_string(),
    signature: signature.to_bytes().to_vec(),
    credentials: vec![/* Optional credentials */],
};

// Verify the authentication
let result = auth_manager.verify_authentication(&response).await?;

if result.authenticated {
    println!("Authentication successful, token: {}", result.token);
} else {
    println!("Authentication failed");
}
```

### Issuing a Credential

```rust
// Create a credential manager
let credential_manager = CredentialManager::new(
    did_resolver.clone(),
    storage.clone(),
    CredentialManagerConfig::default(),
).await?;

// Create credential claims
let mut claims = HashMap::new();
claims.insert("role".to_string(), json!("member"));
claims.insert("permissions".to_string(), json!(["read", "write", "connect"]));
claims.insert("memberSince".to_string(), json!("2023-01-15T00:00:00Z"));

// Issue the credential
let credential = credential_manager.issue_credential(
    "ICNMemberCredential",
    "did:icn:coopA:userX",
    claims,
    &CredentialOptions {
        issuer_did: Some("did:icn:coopA:admin".to_string()),
        validity_days: Some(365),
        store: true,
        additional_contexts: None,
    }
).await?;

println!("Issued credential: {}", credential.id);
```

## Conclusion

The ICN's DID implementation provides a robust foundation for:

1. **Self-sovereign Identity**: Users control their own identifiers and keys
2. **Decentralized Authentication**: No central identity provider
3. **Verifiable Credentials**: Secure, tamper-proof attribute attestations
4. **Federation Support**: Identity that works across cooperative boundaries

This implementation is fully compliant with the W3C DID specification while adding custom extensions for the cooperative network context. It forms the backbone of the ICN's security and authentication model. 