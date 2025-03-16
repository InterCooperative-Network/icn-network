# Peer-to-Peer Authentication in ICN Network

This document describes the authentication mechanisms used for secure peer-to-peer connections in the ICN Network, with a focus on the libp2p integration and how it connects to the broader DID identity system.

## Overview

The ICN Network uses a multilayered authentication approach:

1. **Transport Security**: TLS-based secure connections at the transport layer
2. **Peer Authentication**: Identity verification using libp2p's peer ID system
3. **DID Authentication**: Higher-level authentication tied to the DID identity system
4. **Connection Authorization**: Permission management for allowed connections

This approach ensures that all network communications are:
- **Encrypted**: Protected from eavesdropping
- **Authenticated**: Both parties verify each other's identity
- **Authorized**: Access permissions are enforced based on DID credentials

## Architecture Diagram

```
┌──────────────────────────────────────────────────────────────────┐
│                         Application Layer                         │
└───────────────────────────────┬──────────────────────────────────┘
                                │
┌───────────────────────────────▼──────────────────────────────────┐
│                      DID Authentication Layer                     │
│                                                                   │
│  ┌──────────────────┐  ┌─────────────────┐  ┌────────────────┐   │
│  │ VerifiableCredential │ Federation Auth │  │ Access Control │   │
│  └──────────────────┘  └─────────────────┘  └────────────────┘   │
└───────────────────────────────┬──────────────────────────────────┘
                                │
┌───────────────────────────────▼──────────────────────────────────┐
│                       libp2p Network Layer                        │
│                                                                   │
│  ┌──────────────────┐  ┌─────────────────┐  ┌────────────────┐   │
│  │   Peer Identity  │  │ Noise/TLS Security │  Identify Protocol │   │
│  └──────────────────┘  └─────────────────┘  └────────────────┘   │
└───────────────────────────────┬──────────────────────────────────┘
                                │
┌───────────────────────────────▼──────────────────────────────────┐
│                       Transport Layer                             │
│                                                                   │
│  ┌──────────────────┐  ┌─────────────────┐  ┌────────────────┐   │
│  │       TCP        │  │      QUIC       │  │    WebSocket   │   │
│  └──────────────────┘  └─────────────────┘  └────────────────┘   │
└──────────────────────────────────────────────────────────────────┘
```

## libp2p Authentication

### Peer Identity

Every node in the ICN network has a libp2p identity, which consists of:

1. **Keypair**: A cryptographic keypair (typically Ed25519)
2. **PeerId**: A unique identifier derived from the public key

```rust
/// Initialize a libp2p identity
pub fn create_libp2p_identity() -> Result<Keypair> {
    // Generate a new Ed25519 keypair for libp2p
    let keypair = identity::Keypair::generate_ed25519();
    
    // Get the peer ID from the public key
    let peer_id = PeerId::from(keypair.public());
    
    info!("Created new libp2p identity with peer ID: {}", peer_id);
    
    Ok(keypair)
}
```

### Transport Security

ICN uses `Noise` or `TLS` protocols for transport security:

```rust
/// Configure transport with TLS security
pub fn configure_transport(keypair: &Keypair) -> Result<Transport> {
    // Create a transport with TCP
    let transport = TokioTcpTransport::new(TcpConfig::new().nodelay(true));
    
    // Upgrade with TLS security
    let transport = transport.upgrade(upgrade::Version::V1)
        .authenticate(noise::NoiseConfig::xx(keypair).into_authenticated())
        .multiplex(yamux::YamuxConfig::default())
        .boxed();
        
    Ok(transport)
}
```

### Identify Protocol

The ICN uses libp2p's `Identify` protocol to exchange peer information:

```rust
/// Configure the identify behavior
pub fn configure_identify(keypair: &Keypair) -> identify::Behaviour {
    let local_peer_id = PeerId::from(keypair.public());
    
    identify::Behaviour::new(
        identify::Config::new("/icn/1.0.0".to_string(), keypair.public())
            .with_agent_version(format!("icn-node/{}", env!("CARGO_PKG_VERSION")))
    )
}
```

## Mapping libp2p Identity to DID

The ICN integrates libp2p identities with the DID system by:

1. Recording the libp2p PeerId in the DID document
2. Proving ownership of both identities during authentication

```rust
/// Associate a libp2p PeerId with a DID
pub async fn associate_peer_id_with_did(
    did_manager: &DidManager,
    did: &str,
    keypair: &Keypair
) -> Result<()> {
    // Get the peer ID from the keypair
    let peer_id = PeerId::from(keypair.public());
    
    // Resolve the DID document
    let mut document = did_manager.resolve(did).await?;
    
    // Add the libp2p verification method
    let verification_method = VerificationMethod {
        id: format!("{}#libp2p-key-1", did),
        type_: "Libp2pKey2023".to_string(),
        controller: did.to_string(),
        verification_material: VerificationMaterial::PublicKeyMultibase(
            multibase::encode(multibase::Base::Base58Btc, keypair.public().encode())
        ),
    };
    
    // Add to the document
    document.verification_method.push(verification_method);
    
    // Add service endpoint for libp2p
    document.service.push(Service {
        id: format!("{}#libp2p", did),
        type_: "Libp2pService".to_string(),
        service_endpoint: json!({
            "peerId": peer_id.to_string(),
        }),
    });
    
    // Update the DID document
    did_manager.update_did_document(did, document).await?;
    
    Ok(())
}
```

## P2P Authentication Flow

The authentication flow between two peers involves multiple steps:

### 1. Transport Connection and Encryption

When two nodes connect, they first establish a secure transport connection:

```
┌───────────┐                              ┌───────────┐
│           │                              │           │
│  Node A   │                              │  Node B   │
│           │                              │           │
└─────┬─────┘                              └─────┬─────┘
      │                                          │
      │  1. TCP Connect                          │
      │─────────────────────────────────────────>│
      │                                          │
      │  2. Noise/TLS Handshake                  │
      │<─────────────────────────────────────────│
      │                                          │
      │  3. Secure Channel Established           │
      │<─────────────────────────────────────────│
      │                                          │
```

### 2. libp2p Protocol Negotiation

After establishing a secure connection, nodes negotiate protocols:

```
┌───────────┐                              ┌───────────┐
│           │                              │           │
│  Node A   │                              │  Node B   │
│           │                              │           │
└─────┬─────┘                              └─────┬─────┘
      │                                          │
      │  1. Protocol Negotiation                 │
      │─────────────────────────────────────────>│
      │                                          │
      │  2. Identify Protocol Exchange           │
      │<─────────────────────────────────────────│
      │                                          │
      │  3. Exchange PeerIDs and Agent Info      │
      │<────────────────────────────────────────>│
      │                                          │
```

### 3. DID Authentication Protocol

After basic libp2p authentication, nodes perform DID-based authentication:

```
┌───────────┐                              ┌───────────┐
│           │                              │           │
│  Node A   │                              │  Node B   │
│           │                              │           │
└─────┬─────┘                              └─────┬─────┘
      │                                          │
      │  1. Request DID Authentication           │
      │─────────────────────────────────────────>│
      │                                          │
      │  2. Challenge (Nonce)                    │
      │<─────────────────────────────────────────│
      │                                          │
      │  3. Sign Challenge with DID Key          │
      │─────────────────────────────────────────>│
      │                                          │
      │  4. Verify Signature & PeerId Match      │
      │                                          │
      │  5. Authentication Result                │
      │<─────────────────────────────────────────│
      │                                          │
      │  6. Mutual Authentication (Repeat 1-5)   │
      │<────────────────────────────────────────>│
      │                                          │
```

## Protocol Implementation

### Authentication Protocol Handler

```rust
/// Implementation of the DID authentication protocol for libp2p
pub struct DidAuthProtocolHandler {
    /// DID resolver
    did_resolver: Arc<DidResolver>,
    
    /// Authentication manager
    auth_manager: Arc<AuthenticationManager>,
    
    /// Local keypair
    local_keypair: Keypair,
    
    /// Local DID
    local_did: String,
}

impl DidAuthProtocolHandler {
    /// Create a new protocol handler
    pub fn new(
        did_resolver: Arc<DidResolver>,
        auth_manager: Arc<AuthenticationManager>,
        local_keypair: Keypair,
        local_did: String,
    ) -> Self {
        Self {
            did_resolver,
            auth_manager,
            local_keypair,
            local_did,
        }
    }
    
    /// Handle inbound authentication request
    pub async fn handle_inbound(
        &self,
        peer_id: PeerId,
        request: AuthRequest,
    ) -> Result<AuthResponse> {
        match request {
            AuthRequest::InitAuth { did } => {
                // Generate challenge
                let challenge = self.auth_manager.begin_authentication(&did).await?;
                
                // Return challenge
                Ok(AuthResponse::Challenge { challenge })
            }
            
            AuthRequest::VerifyAuth { challenge_id, signature, key_id } => {
                // Create auth response
                let auth_response = AuthenticationResponse {
                    challenge_id,
                    key_id,
                    signature: signature.to_vec(),
                    credentials: vec![],
                };
                
                // Verify the authentication
                let result = self.auth_manager.verify_authentication(&auth_response).await?;
                
                // Verify that the peer ID matches the DID
                self.verify_peer_id_matches_did(peer_id, &result.did).await?;
                
                // Return the result
                Ok(AuthResponse::AuthResult { result })
            }
        }
    }
    
    /// Initiate authentication with a peer
    pub async fn authenticate_peer(
        &self,
        peer_id: PeerId,
        target_did: &str,
    ) -> Result<AuthenticationResult> {
        // Send initial auth request
        let init_request = AuthRequest::InitAuth { 
            did: self.local_did.clone() 
        };
        
        // Send request and get challenge
        let challenge = match self.send_request(peer_id, init_request).await? {
            AuthResponse::Challenge { challenge } => challenge,
            _ => return Err(Error::protocol("Unexpected response")),
        };
        
        // Sign the challenge with our DID key
        let key_id = format!("{}#keys-1", self.local_did);
        let message = format!("{}:{}:{}", challenge.did, challenge.nonce, challenge.timestamp);
        
        // Get the DID keypair
        let did_keypair = self.auth_manager.get_keypair(&self.local_did, &key_id).await?;
        
        // Sign the message
        let signature = did_keypair.sign(message.as_bytes())?;
        
        // Create verify request
        let verify_request = AuthRequest::VerifyAuth {
            challenge_id: challenge.id,
            signature: signature.to_bytes().to_vec(),
            key_id,
        };
        
        // Send verification and get result
        match self.send_request(peer_id, verify_request).await? {
            AuthResponse::AuthResult { result } => Ok(result),
            _ => Err(Error::protocol("Unexpected response")),
        }
    }
    
    /// Verify that a peer ID matches a DID
    async fn verify_peer_id_matches_did(&self, peer_id: PeerId, did: &str) -> Result<()> {
        // Resolve the DID document
        let document = self.did_resolver.resolve(did).await?;
        
        // Find the libp2p service
        let service = document.service.iter()
            .find(|s| s.type_ == "Libp2pService")
            .ok_or_else(|| Error::not_found("Libp2p service not found in DID document"))?;
        
        // Get the peer ID from the service
        let service_peer_id = match &service.service_endpoint {
            serde_json::Value::Object(obj) => {
                obj.get("peerId")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| Error::not_found("Peer ID not found in service endpoint"))?
            },
            _ => return Err(Error::invalid_data("Invalid service endpoint format")),
        };
        
        // Parse the peer ID
        let expected_peer_id = PeerId::from_str(service_peer_id)
            .map_err(|_| Error::invalid_data("Invalid peer ID format"))?;
        
        // Compare peer IDs
        if peer_id != expected_peer_id {
            return Err(Error::unauthorized(format!(
                "Peer ID mismatch: expected {}, got {}",
                expected_peer_id, peer_id
            )));
        }
        
        Ok(())
    }
    
    // Other implementation methods...
}
```

## Connection Authorization

After authentication, the ICN applies authorization rules to determine if the connection should be allowed:

```rust
/// Authorization for peer connections
pub struct PeerAuthorization {
    /// DID resolver
    did_resolver: Arc<DidResolver>,
    
    /// Credential manager
    credential_manager: Arc<CredentialManager>,
    
    /// Federation manager
    federation_manager: Arc<FederationManager>,
}

impl PeerAuthorization {
    /// Check if a peer is authorized to connect
    pub async fn is_authorized(
        &self,
        local_did: &str,
        peer_did: &str,
        authentication_result: &AuthenticationResult
    ) -> Result<AuthorizationResult> {
        // Get local cooperative ID
        let local_coop_id = self.extract_coop_id(local_did)?;
        
        // Get peer cooperative ID
        let peer_coop_id = self.extract_coop_id(peer_did)?;
        
        // Check if same cooperative
        if local_coop_id == peer_coop_id {
            return self.authorize_same_coop(local_did, peer_did, authentication_result).await;
        }
        
        // Check federation
        self.authorize_federation(local_did, peer_did, authentication_result).await
    }
    
    /// Authorize a peer from the same cooperative
    async fn authorize_same_coop(
        &self,
        local_did: &str,
        peer_did: &str,
        auth_result: &AuthenticationResult
    ) -> Result<AuthorizationResult> {
        // Check node credentials
        if !self.has_node_credentials(auth_result) {
            return Ok(AuthorizationResult {
                authorized: false,
                reason: Some("Missing required node credentials".to_string()),
            });
        }
        
        // Check if in allowed list
        let is_allowed = self.check_allowed_nodes(local_did, peer_did).await?;
        
        if !is_allowed {
            return Ok(AuthorizationResult {
                authorized: false,
                reason: Some("Node not in allowed connections list".to_string()),
            });
        }
        
        Ok(AuthorizationResult {
            authorized: true,
            reason: None,
        })
    }
    
    /// Authorize a peer from a different cooperative (federation)
    async fn authorize_federation(
        &self,
        local_did: &str,
        peer_did: &str,
        auth_result: &AuthenticationResult
    ) -> Result<AuthorizationResult> {
        // Extract cooperative IDs
        let local_coop_id = self.extract_coop_id(local_did)?;
        let peer_coop_id = self.extract_coop_id(peer_did)?;
        
        // Check if federation exists between cooperatives
        let federation = self.federation_manager.get_federation(local_coop_id, peer_coop_id).await;
        
        if federation.is_err() {
            return Ok(AuthorizationResult {
                authorized: false,
                reason: Some("No federation agreement between cooperatives".to_string()),
            });
        }
        
        // Check federation credentials
        if !self.has_federation_credentials(auth_result, &federation?) {
            return Ok(AuthorizationResult {
                authorized: false,
                reason: Some("Missing required federation credentials".to_string()),
            });
        }
        
        // Check if peer is a designated federation node
        if !self.is_federation_node(peer_did).await? {
            return Ok(AuthorizationResult {
                authorized: false,
                reason: Some("Peer is not a designated federation node".to_string()),
            });
        }
        
        Ok(AuthorizationResult {
            authorized: true,
            reason: None,
        })
    }
    
    /// Check if the authentication result has valid node credentials
    fn has_node_credentials(&self, auth_result: &AuthenticationResult) -> bool {
        auth_result.verified_credentials.iter().any(|cred| {
            cred.types.contains(&"ICNNodeCredential".to_string())
        })
    }
    
    // Other implementation methods...
}
```

## Security Considerations

### 1. Defense in Depth

ICN's authentication uses multiple layers of security:
- Transport encryption (TLS/Noise)
- libp2p peer authentication
- DID-based identity verification
- Credential-based authorization

### 2. Key Management

- **Separate Keys**: Different keys for different purposes (libp2p, DID authentication)
- **Key Isolation**: Prevents compromise of one key from affecting others
- **Key Rotation**: Regular rotation of keys with proper transition periods

### 3. Potential Attacks and Mitigations

| Attack | Mitigation |
|--------|------------|
| Man-in-the-Middle | Transport encryption, mutual authentication |
| Identity Spoofing | Cryptographic verification of peer identity |
| Replay Attacks | Challenge-response with nonce and timestamps |
| Credential Forgery | Cryptographic verification of credentials |
| Denial of Service | Rate limiting, connection prioritization |

## Integration with External Identity Systems

The ICN can integrate with external identity systems:

### Web3 Wallet Authentication

```rust
/// Authenticate using a Web3 wallet
pub async fn authenticate_with_web3_wallet(
    did_manager: &DidManager,
    ethereum_address: &str,
    signature: &[u8],
    message: &str
) -> Result<String> {
    // Verify Ethereum signature
    let recovered_address = verify_ethereum_signature(message, signature)?;
    
    if recovered_address != ethereum_address {
        return Err(Error::unauthorized("Invalid Ethereum signature"));
    }
    
    // Check if DID exists for this address
    let did = match find_did_for_ethereum_address(did_manager, ethereum_address).await {
        Ok(did) => did,
        Err(_) => {
            // Create a new DID for this Ethereum address
            create_did_for_ethereum_address(did_manager, ethereum_address).await?
        }
    };
    
    Ok(did)
}
```

### OAuth Integration

```rust
/// Authenticate using OAuth
pub async fn authenticate_with_oauth(
    did_manager: &DidManager,
    provider: OAuthProvider,
    token: &str
) -> Result<String> {
    // Verify OAuth token
    let user_info = verify_oauth_token(provider, token).await?;
    
    // Check if DID exists for this user
    let did = match find_did_for_oauth_user(did_manager, &provider, &user_info.id).await {
        Ok(did) => did,
        Err(_) => {
            // Create a new DID for this OAuth user
            create_did_for_oauth_user(did_manager, &provider, &user_info).await?
        }
    };
    
    Ok(did)
}
```

## Conclusion

The ICN Network's peer-to-peer authentication system provides:

1. **Strong Security**: Multi-layered authentication with cryptographic verification
2. **Identity Integration**: Seamless connection between libp2p and the DID system
3. **Federation Support**: Secure authentication across cooperative boundaries
4. **Extensibility**: Integration with external identity systems

This comprehensive approach ensures that all connections in the network are secure, authenticated, and properly authorized, while maintaining compatibility with web standards like DIDs and verifiable credentials. 