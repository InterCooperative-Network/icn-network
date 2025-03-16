# ICN Federation Guide

This guide explains how federation works in the ICN network, allowing different cooperatives to securely connect and share resources while maintaining their sovereignty.

## What is Federation?

Federation in the ICN network refers to the ability of independent cooperative networks to:

1. **Discover** each other across organizational boundaries
2. **Authenticate** users and services from other cooperatives
3. **Share** specific resources and services based on federation agreements
4. **Maintain** sovereignty and independent governance

The federation model is built on principles of mutual consent, transparency, and technical interoperability.

## Federation Architecture

### High-Level Overview

```
┌───────────────────────┐           ┌───────────────────────┐
│                       │           │                       │
│   Cooperative A       │◄─────────►│   Cooperative B       │
│                       │  Federation│                       │
└───────────┬───────────┘  Agreement └───────────┬───────────┘
            │                                    │
            │                                    │
            ▼                                    ▼
┌───────────────────────┐           ┌───────────────────────┐
│                       │           │                       │
│  Federation Gateway   │◄─────────►│  Federation Gateway   │
│                       │  Federation│                       │
└───────────┬───────────┘  Protocol └───────────┬───────────┘
            │                                    │
            │                                    │
            ▼                                    ▼
┌───────────────────────┐           ┌───────────────────────┐
│   Local Services      │           │   Local Services      │
│   and Resources       │           │   and Resources       │
│                       │           │                       │
└───────────────────────┘           └───────────────────────┘
```

### Federation Gateway Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│                      Federation Gateway                          │
│                                                                  │
│  ┌────────────────┐  ┌────────────────┐  ┌────────────────┐     │
│  │  Federation    │  │  Identity      │  │  Policy        │     │
│  │  Protocol      │  │  Bridge        │  │  Enforcement   │     │
│  └────────┬───────┘  └────────┬───────┘  └────────┬───────┘     │
│           │                   │                   │              │
│           └───────────┬───────┴───────────┬───────┘              │
│                       │                   │                      │
│  ┌────────────────────▼────┐  ┌──────────▼─────────────────┐    │
│  │  Federation Registry    │  │  Federation Credentials    │    │
│  └─────────────────────────┘  └────────────────────────────┘    │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

## Federation Setup Process

### 1. Federation Agreement

Before technical implementation, cooperatives establish a federation agreement that covers:

- **Governance**: How decisions about the federation are made
- **Shared Services**: Which services will be accessible
- **Resource Allocation**: How resources are allocated and accounted for
- **Privacy Policies**: How user data is handled across boundaries
- **Security Standards**: Minimum security requirements
- **Termination Conditions**: Process for ending federation

This agreement is recorded on-chain as a smart contract that both cooperatives sign.

### 2. Technical Configuration

#### Federation Registry Setup

```rust
/// Initialize a federation relationship
pub async fn initialize_federation(
    config: &FederationConfig,
    local_coop_id: &str,
    remote_coop_id: &str,
    agreement_id: &str
) -> Result<FederationInfo> {
    // Create federation information
    let federation_info = FederationInfo {
        id: Uuid::new_v4().to_string(),
        local_cooperative_id: local_coop_id.to_string(),
        remote_cooperative_id: remote_coop_id.to_string(),
        agreement_id: agreement_id.to_string(),
        created_at: chrono::Utc::now(),
        status: FederationStatus::Initializing,
        gateway_endpoints: Vec::new(),
        shared_services: Vec::new(),
        trust_anchors: Vec::new(),
    };
    
    // Store in local registry
    config.federation_registry.store_federation(&federation_info).await?;
    
    // Create trust anchors
    initialize_trust_anchors(config, &federation_info).await?;
    
    Ok(federation_info)
}
```

#### Gateway Configuration

```rust
/// Configure federation gateway
pub async fn configure_federation_gateway(
    config: &FederationConfig,
    federation_info: &FederationInfo
) -> Result<FederationGateway> {
    // Create gateway configuration
    let gateway_config = FederationGatewayConfig {
        federation_id: federation_info.id.clone(),
        listen_address: config.listen_address.clone(),
        public_address: config.public_address.clone(),
        tls_config: config.tls_config.clone(),
        max_connections: config.max_connections,
        connection_timeout: config.connection_timeout,
    };
    
    // Initialize the gateway
    let gateway = FederationGateway::new(
        gateway_config,
        config.did_resolver.clone(),
        config.auth_manager.clone(),
        config.policy_engine.clone(),
    ).await?;
    
    // Register gateway endpoints
    let endpoints = vec![
        GatewayEndpoint {
            address: config.public_address.clone(),
            protocol: "https".to_string(),
            api_version: "v1".to_string(),
        }
    ];
    
    config.federation_registry.update_gateway_endpoints(
        &federation_info.id,
        &endpoints
    ).await?;
    
    Ok(gateway)
}
```

### 3. Federation Protocol Handshake

When establishing a federation link, gateways perform a handshake protocol:

```
┌───────────────┐                              ┌───────────────┐
│               │                              │               │
│  Gateway A    │                              │  Gateway B    │
│               │                              │               │
└─────┬─────────┘                              └─────┬─────────┘
      │                                              │
      │  1. Federation Request                       │
      │  (Federation ID, DID, Agreement ID)          │
      │─────────────────────────────────────────────>│
      │                                              │
      │  2. Challenge                                │
      │  (Nonce, Timestamp)                          │
      │<─────────────────────────────────────────────│
      │                                              │
      │  3. Signed Challenge + Gateway Endpoints     │
      │─────────────────────────────────────────────>│
      │                                              │
      │  4. Verify Agreement (on-chain)              │
      │                                              │
      │  5. Send Trust Anchors                       │
      │<─────────────────────────────────────────────│
      │                                              │
      │  6. Send Federation Services                 │
      │─────────────────────────────────────────────>│
      │                                              │
      │  7. Federation Established                   │
      │<─────────────────────────────────────────────│
      │                                              │
```

## Cross-Cooperative Authentication

### Identity Bridging

Federation enables identity bridging, where DIDs from one cooperative can be authenticated in another:

```rust
/// Bridge a DID from another cooperative
pub async fn bridge_remote_did(
    federation_manager: &FederationManager,
    remote_did: &str,
    auth_token: &str
) -> Result<BridgedIdentity> {
    // Extract cooperative ID from DID
    let coop_id = extract_coop_id(remote_did)?;
    
    // Get federation information
    let federation = federation_manager.get_federation_by_coop(coop_id).await?;
    
    // Verify the token with remote gateway
    let verification = federation_manager.verify_remote_token(
        &federation, 
        remote_did, 
        auth_token
    ).await?;
    
    if !verification.is_valid {
        return Err(Error::unauthorized("Invalid remote authentication token"));
    }
    
    // Create bridged identity
    let bridged_identity = BridgedIdentity {
        remote_did: remote_did.to_string(),
        federation_id: federation.id.clone(),
        verified_claims: verification.claims,
        permissions: derive_permissions_from_claims(&verification.claims, &federation),
        created_at: chrono::Utc::now(),
        expires_at: chrono::Utc::now() + chrono::Duration::hours(8),
    };
    
    // Store bridged identity
    federation_manager.store_bridged_identity(&bridged_identity).await?;
    
    Ok(bridged_identity)
}
```

### Authentication Flow

The cross-cooperative authentication flow works as follows:

```
┌───────────┐          ┌───────────┐          ┌───────────┐
│           │          │           │          │           │
│   User    │          │ Gateway A │          │ Gateway B │
│           │          │           │          │           │
└─────┬─────┘          └─────┬─────┘          └─────┬─────┘
      │                      │                      │
      │ 1. Authenticate      │                      │
      │ with Coop A          │                      │
      │─────────────────────>│                      │
      │                      │                      │
      │ 2. Coop A Token      │                      │
      │<─────────────────────│                      │
      │                      │                      │
      │ 3. Request Access    │                      │
      │ to Coop B Resource   │                      │
      │─────────────────────>│                      │
      │                      │                      │
      │                      │ 4. Federation        │
      │                      │ Authentication       │
      │                      │─────────────────────>│
      │                      │                      │
      │                      │ 5. Verify Token      │
      │                      │ & Check Permissions  │
      │                      │                      │
      │                      │ 6. Create Bridged    │
      │                      │ Identity             │
      │                      │                      │
      │                      │ 7. Bridged Token     │
      │                      │<─────────────────────│
      │                      │                      │
      │ 8. Bridged Token     │                      │
      │<─────────────────────│                      │
      │                      │                      │
      │ 9. Access Resource   │                      │
      │ with Bridged Token   │                      │
      │─────────────────────────────────────────────>│
      │                      │                      │
```

## Resource Sharing

### Service Discovery Across Federations

Federation enables service discovery across cooperative boundaries:

```rust
/// Discover services across federation
pub async fn discover_federated_services(
    name_resolver: &NameResolver,
    federation_manager: &FederationManager,
    query: &ServiceQuery
) -> Result<Vec<FederatedService>> {
    // Check if this is a query for federated services
    if !query.include_federated {
        return Ok(Vec::new());
    }
    
    // Get all active federations
    let federations = federation_manager.get_active_federations().await?;
    
    let mut federated_services = Vec::new();
    
    // Query each federation
    for federation in federations {
        // Skip if federation doesn't match filters
        if let Some(coop_id) = &query.cooperative_id {
            if &federation.remote_cooperative_id != coop_id {
                continue;
            }
        }
        
        // Query the federation gateway
        match federation_manager.query_remote_services(&federation, query).await {
            Ok(services) => {
                // Add federation context to services
                for service in services {
                    federated_services.push(FederatedService {
                        service,
                        federation_id: federation.id.clone(),
                        cooperative_id: federation.remote_cooperative_id.clone(),
                    });
                }
            },
            Err(e) => {
                warn!("Failed to query services from federation {}: {}",
                    federation.id, e);
                continue;
            }
        }
    }
    
    Ok(federated_services)
}
```

### DHT-Based Name Resolution Across Federations

Federation extends DHT-based name resolution across cooperative boundaries:

```rust
/// Resolve name across federation
pub async fn resolve_federated_name(
    name_resolver: &NameResolver,
    federation_manager: &FederationManager,
    name: &str
) -> Result<NameResolution> {
    // Try local resolution first
    match name_resolver.resolve_name(name).await {
        Ok(resolution) => return Ok(resolution),
        Err(_) => {
            // If local resolution fails, try federation
            if let Some(coop_id) = extract_cooperative_from_name(name) {
                let federations = federation_manager.get_federations_by_coop(coop_id).await?;
                
                // Try each federation
                for federation in federations {
                    match federation_manager.resolve_remote_name(&federation, name).await {
                        Ok(resolution) => return Ok(resolution),
                        Err(_) => continue,
                    }
                }
            }
        }
    }
    
    Err(Error::not_found(format!("Name {} not found locally or in federations", name)))
}
```

## Federation Trust Model

The ICN federation uses a combination of technical measures to establish trust:

### 1. Trust Anchors

Each cooperative designates a set of DIDs as "trust anchors" that are authorized to issue credentials:

```rust
/// Initialize trust anchors for federation
pub async fn initialize_trust_anchors(
    config: &FederationConfig,
    federation_info: &FederationInfo
) -> Result<Vec<TrustAnchor>> {
    // Create trust anchors
    let trust_anchors = vec![
        TrustAnchor {
            id: Uuid::new_v4().to_string(),
            did: config.cooperative_did.clone(),
            type_: TrustAnchorType::Primary,
            purpose: vec![TrustPurpose::Authentication, TrustPurpose::ServiceRegistry],
            public_key: config.cooperative_key.public_key(),
        },
        TrustAnchor {
            id: Uuid::new_v4().to_string(),
            did: config.gateway_did.clone(),
            type_: TrustAnchorType::Gateway,
            purpose: vec![TrustPurpose::Authentication, TrustPurpose::FederationProtocol],
            public_key: config.gateway_key.public_key(),
        }
    ];
    
    // Store trust anchors
    for anchor in &trust_anchors {
        config.federation_registry.store_trust_anchor(
            &federation_info.id,
            anchor
        ).await?;
    }
    
    Ok(trust_anchors)
}
```

### 2. Verifiable Credentials for Federation

Federation-specific credentials grant access to resources:

```json
{
  "@context": [
    "https://www.w3.org/2018/credentials/v1",
    "https://w3id.org/security/suites/ed25519-2020/v1",
    "https://icn.coop/federation/v1"
  ],
  "id": "did:icn:coopA:cred-fed-1234",
  "type": ["VerifiableCredential", "FederationAccessCredential"],
  "issuer": "did:icn:coopA:admin",
  "issuanceDate": "2023-03-10T12:00:00Z",
  "expirationDate": "2024-03-10T12:00:00Z",
  "credentialSubject": {
    "id": "did:icn:coopB:userX",
    "federationId": "fed-abcd-1234",
    "allowedServices": ["storage", "compute", "messaging"],
    "resourceQuota": {
      "storageGB": 100,
      "computeHours": 50,
      "bandwidthGB": 500
    },
    "accessLevel": "standard"
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

### 3. Blockchain Verification

Federation agreements are verified on-chain:

```rust
/// Verify federation agreement on-chain
pub async fn verify_federation_agreement(
    blockchain_client: &BlockchainClient,
    agreement_id: &str,
    local_coop_id: &str,
    remote_coop_id: &str
) -> Result<FederationAgreement> {
    // Get agreement from blockchain
    let agreement = blockchain_client.get_federation_agreement(agreement_id).await?;
    
    // Verify parties
    if !agreement.parties.contains(&local_coop_id.to_string()) ||
       !agreement.parties.contains(&remote_coop_id.to_string()) {
        return Err(Error::invalid_data(
            "Federation agreement does not include both cooperatives"
        ));
    }
    
    // Verify status
    if agreement.status != FederationAgreementStatus::Active {
        return Err(Error::invalid_state(
            format!("Federation agreement is not active: {:?}", agreement.status)
        ));
    }
    
    // Verify signatures
    let local_signature = agreement.signatures.get(local_coop_id)
        .ok_or_else(|| Error::invalid_data("Missing local signature on agreement"))?;
        
    let remote_signature = agreement.signatures.get(remote_coop_id)
        .ok_or_else(|| Error::invalid_data("Missing remote signature on agreement"))?;
    
    // Verify local signature
    blockchain_client.verify_signature(
        local_coop_id,
        &agreement.content_hash,
        local_signature
    ).await?;
    
    // Verify remote signature
    blockchain_client.verify_signature(
        remote_coop_id,
        &agreement.content_hash,
        remote_signature
    ).await?;
    
    Ok(agreement)
}
```

## Federation Policy Enforcement

Policies control what resources and services are accessible across federations:

```rust
/// Federation policy enforcement
pub struct FederationPolicyEnforcer {
    /// Policy engine
    policy_engine: Arc<PolicyEngine>,
    
    /// Federation registry
    federation_registry: Arc<FederationRegistry>,
    
    /// Resource tracker
    resource_tracker: Arc<ResourceTracker>,
}

impl FederationPolicyEnforcer {
    /// Check if a federated request is allowed
    pub async fn check_request(
        &self,
        federation_id: &str,
        user_did: &str,
        service_id: &str,
        action: &str,
        resource: &str
    ) -> Result<PolicyDecision> {
        // Get federation info
        let federation = self.federation_registry.get_federation(federation_id).await?;
        
        // Get user credentials
        let credentials = self.federation_registry.get_user_credentials(
            federation_id,
            user_did
        ).await?;
        
        // Check if service is shared
        let service_shared = federation.shared_services.iter()
            .any(|s| s.id == service_id);
            
        if !service_shared {
            return Ok(PolicyDecision {
                allowed: false,
                reason: Some("Service is not shared in this federation".to_string()),
                conditions: None,
            });
        }
        
        // Check resource limits
        let resource_check = self.resource_tracker.check_limits(
            federation_id,
            user_did,
            resource,
            action
        ).await?;
        
        if !resource_check.allowed {
            return Ok(PolicyDecision {
                allowed: false,
                reason: Some(format!("Resource limit exceeded: {}", 
                    resource_check.reason.unwrap_or_default())),
                conditions: None,
            });
        }
        
        // Evaluate policy
        let policy_context = PolicyContext {
            subject: user_did.to_string(),
            resource: resource.to_string(),
            action: action.to_string(),
            environment: json!({
                "federation_id": federation_id,
                "service_id": service_id,
                "timestamp": chrono::Utc::now().timestamp(),
            }),
            credentials,
        };
        
        let decision = self.policy_engine.evaluate(
            "federation",
            &policy_context
        ).await?;
        
        Ok(decision)
    }
}
```

## Resource Accounting and Settlement

Federation includes mechanisms for tracking and settling resource usage:

```rust
/// Track federation resource usage
pub async fn track_resource_usage(
    resource_tracker: &ResourceTracker,
    federation_id: &str,
    user_did: &str,
    service_id: &str,
    resource_type: &str,
    amount: f64
) -> Result<()> {
    // Create usage record
    let usage = ResourceUsage {
        id: Uuid::new_v4().to_string(),
        federation_id: federation_id.to_string(),
        user_did: user_did.to_string(),
        service_id: service_id.to_string(),
        resource_type: resource_type.to_string(),
        amount,
        timestamp: chrono::Utc::now(),
    };
    
    // Record usage
    resource_tracker.record_usage(&usage).await?;
    
    // Update user quota
    resource_tracker.update_user_quota(
        federation_id,
        user_did,
        resource_type,
        amount
    ).await?;
    
    // Check if settlement is needed
    let should_settle = resource_tracker.should_trigger_settlement(
        federation_id,
        resource_type
    ).await?;
    
    if should_settle {
        // Trigger settlement asynchronously
        tokio::spawn(async move {
            match resource_tracker.trigger_settlement(federation_id).await {
                Ok(_) => info!("Federation settlement completed for {}", federation_id),
                Err(e) => error!("Federation settlement failed: {}", e),
            }
        });
    }
    
    Ok(())
}
```

## Security Considerations

### 1. Cross-Cooperative Data Protection

The ICN federation model includes several protections for user data:

- **Selective Disclosure**: Only necessary claims are shared across federation
- **User Consent**: Users must consent to federated authentication
- **Data Minimization**: Services only receive claims needed for access
- **Auditability**: All cross-federation requests are logged and auditable

### 2. Federation Revocation

Federations can be revoked if security issues arise:

```rust
/// Revoke federation due to security issue
pub async fn emergency_revoke_federation(
    federation_manager: &FederationManager,
    federation_id: &str,
    reason: &str
) -> Result<()> {
    // Get federation
    let mut federation = federation_manager.get_federation(federation_id).await?;
    
    // Update status
    federation.status = FederationStatus::Revoked;
    
    // Record revocation
    federation_manager.registry.update_federation(&federation).await?;
    
    // Record security incident
    federation_manager.security_log.record_incident(
        SecurityIncidentType::FederationRevoked,
        federation_id,
        reason
    ).await?;
    
    // Notify remote gateway
    let _ = federation_manager.notify_remote_gateway(
        &federation,
        FederationNotification::Revocation {
            federation_id: federation_id.to_string(),
            reason: reason.to_string(),
            timestamp: chrono::Utc::now(),
        }
    ).await;
    
    // Revoke all bridged identities
    federation_manager.revoke_all_bridged_identities(federation_id).await?;
    
    Ok(())
}
```

### 3. Vulnerability Management

Federation partners agree to responsible vulnerability disclosure:

```rust
/// Report security vulnerability to federation partner
pub async fn report_federation_vulnerability(
    federation_manager: &FederationManager,
    federation_id: &str,
    vulnerability: &SecurityVulnerability
) -> Result<()> {
    // Get federation
    let federation = federation_manager.get_federation(federation_id).await?;
    
    // Create encrypted vulnerability report
    let report = federation_manager.create_encrypted_vulnerability_report(
        &federation,
        vulnerability
    ).await?;
    
    // Send to remote gateway
    federation_manager.send_vulnerability_report(
        &federation,
        &report
    ).await?;
    
    // Log report
    federation_manager.security_log.record_vulnerability_report(
        federation_id,
        &vulnerability.id
    ).await?;
    
    Ok(())
}
```

## Setting Up a Federation: Step-by-Step Guide

### 1. Governance Setup

1. Define the federation purpose and scope
2. Determine governance structure (voting rights, etc.)
3. Agree on resource sharing terms
4. Define dispute resolution procedures
5. Draft and sign federation agreement

### 2. Technical Setup

#### Configure Federation on Cooperative A

```bash
# Initialize federation configuration
icn-cli federation init \
  --coop-id coopA \
  --gateway-address https://gateway.coopA.icn:8443 \
  --agreement-file ./federation-agreement.json

# Generate federation keys
icn-cli federation generate-keys \
  --output ./federation-keys.json

# Register federation agreement on-chain
icn-cli federation register-agreement \
  --agreement-file ./federation-agreement.json
```

#### Configure Federation on Cooperative B

```bash
# Initialize federation configuration
icn-cli federation init \
  --coop-id coopB \
  --gateway-address https://gateway.coopB.icn:8443 \
  --agreement-file ./federation-agreement.json

# Generate federation keys
icn-cli federation generate-keys \
  --output ./federation-keys.json

# Register federation agreement on-chain
icn-cli federation register-agreement \
  --agreement-file ./federation-agreement.json
```

#### Establish Federation Link

```bash
# On Cooperative A
icn-cli federation establish-link \
  --remote-coop coopB \
  --remote-gateway https://gateway.coopB.icn:8443 \
  --agreement-id fed-agreement-1234

# On Cooperative B
# Federation link request will be automatically processed
```

### 3. Service Configuration

```bash
# Share a service in federation
icn-cli federation share-service \
  --federation-id fed-1234 \
  --service-id storage-service \
  --access-level read-write \
  --quota-config ./storage-quota.json

# Configure federation policies
icn-cli federation set-policy \
  --federation-id fed-1234 \
  --policy-file ./federation-policies.json
```

## Monitoring and Managing Federation

### Federation Health Monitoring

```bash
# Check federation status
icn-cli federation status \
  --federation-id fed-1234

# Output:
# Federation Status: ACTIVE
# Remote Cooperative: coopB
# Gateway Status: ONLINE
# Last Heartbeat: 2023-06-15T14:23:45Z
# Services Shared: 5
# Current Active Users: 17
# Resource Usage:
#   - Storage: 45.3 GB / 100 GB
#   - Bandwidth: 120.7 GB / 500 GB
#   - Compute: 23.5 hrs / 100 hrs
```

### Resource Utilization Reporting

```bash
# Generate federation usage report
icn-cli federation usage-report \
  --federation-id fed-1234 \
  --start-date 2023-06-01 \
  --end-date 2023-06-15 \
  --output ./federation-usage.pdf
```

## Conclusion

The ICN federation model enables:

1. **Sovereignty**: Each cooperative maintains full control over their network
2. **Interoperability**: Services can be accessed across cooperative boundaries
3. **Trust**: Strong cryptographic verification of identities and permissions
4. **Accountability**: Clear tracking of resource usage and settlements

By federating, cooperatives can scale their networks and services while preserving their independence and governance structures. This federation approach creates a more resilient, democratic internet infrastructure that respects the autonomy of each participant while enabling powerful collaborative networking. 