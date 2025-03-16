# DHT-based Name Resolution

This document explains the DHT-based name resolution system used in the ICN network, which provides a decentralized alternative to traditional DNS.

## Overview

The ICN network uses a distributed hash table (DHT) to resolve human-readable names to network addresses, replacing the need for centralized DNS. This system:

1. **Resolves ICN-specific domains** (e.g., `database.coopA.icn`)
2. **Maps services to DIDs and IP addresses**
3. **Provides blockchain fallback** for authoritative validation
4. **Enables peer-to-peer service discovery**

## Architecture

```
┌───────────────────────────────────┐
│                                   │
│          Application Layer        │
│                                   │
├───────────────────────────────────┤
│                                   │
│       Name Resolution System      │
│                                   │
├───────────┬───────────┬───────────┤
│           │           │           │
│  Local    │    DHT    │ Blockchain│
│  Cache    │  Lookup   │  Fallback │
│           │           │           │
└───────────┴───────────┴───────────┘
```

### Component Flow

```
                    ┌───────────────┐
                    │               │
                    │  Application  │
                    │               │
                    └───────┬───────┘
                            │
                            ▼
                   ┌────────────────┐
                   │                │
                   │ Name Resolver  │
                   │                │
                   └───┬────────┬───┘
                       │        │
          ┌────────────┘        └────────────┐
          ▼                                  ▼
┌─────────────────┐                 ┌─────────────────┐
│                 │                 │                 │
│   Local Cache   │                 │   DHT Service   │
│                 │                 │                 │
└─────────┬───────┘                 └────────┬────────┘
          │                                  │
          │ Cache Miss                       │ Not Found
          └────────────────┬─────────────────┘
                           │
                           ▼
                  ┌─────────────────┐
                  │                 │
                  │   Blockchain    │
                  │    Fallback     │
                  │                 │
                  └─────────────────┘
```

## Name Format and Resolution Process

### Domain Format

ICN uses a hierarchical name format:

```
<service>.<coop>.<TLD>
```

Where:
- `<service>` is the service name
- `<coop>` is the cooperative ID
- `<TLD>` is always "icn" for ICN network resources

Examples:
- `database.coopA.icn`
- `auth.federation1.icn`
- `storage.coopB.icn`

### Resolution Process

1. **Local Cache Check**: Check if the name is in the local cache
2. **DHT Lookup**: Query the DHT for the name
3. **Blockchain Fallback**: If not found in DHT, query the blockchain
4. **Result Caching**: Cache successful resolutions

## Implementation Details

### Name Resolution Manager

```rust
/// Configuration for the name resolution system
#[derive(Clone, Debug)]
pub struct NameResolutionConfig {
    /// TTL for cache entries (in seconds)
    pub cache_ttl: u64,
    
    /// Maximum cache size
    pub max_cache_size: usize,
    
    /// Enable blockchain fallback
    pub enable_blockchain_fallback: bool,
    
    /// DHT record prefix
    pub dht_prefix: String,
}

impl Default for NameResolutionConfig {
    fn default() -> Self {
        Self {
            cache_ttl: 3600, // 1 hour
            max_cache_size: 1000,
            enable_blockchain_fallback: true,
            dht_prefix: "name:".to_string(),
        }
    }
}

/// Name resolution manager
pub struct NameResolutionManager {
    /// Configuration
    config: NameResolutionConfig,
    
    /// DHT service
    dht: Arc<DhtService>,
    
    /// Blockchain client (optional)
    blockchain: Option<Arc<BlockchainClient>>,
    
    /// Local cache
    cache: Arc<RwLock<LruCache<String, NameResolutionResult>>>,
    
    /// DID resolver
    did_resolver: Arc<DidResolver>,
}

/// Resolution result
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NameResolutionResult {
    /// The resolved name
    pub name: String,
    
    /// The DID associated with this name
    pub did: Option<String>,
    
    /// Network addresses for this name
    pub addresses: Vec<NameAddress>,
    
    /// When this resolution result expires
    pub expires_at: u64,
    
    /// Source of this resolution
    pub source: ResolutionSource,
}

/// Network address
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NameAddress {
    /// Address type (ipv6, ipv4, multiaddr)
    pub address_type: String,
    
    /// The actual address
    pub address: String,
    
    /// Port (if applicable)
    pub port: Option<u16>,
    
    /// Transport protocol (tcp, udp, etc.)
    pub transport: Option<String>,
    
    /// Priority (lower is higher priority)
    pub priority: u8,
}

/// Resolution source
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ResolutionSource {
    /// DHT record
    Dht,
    /// Blockchain record
    Blockchain,
    /// Local entry
    Local,
}
```

### Name Resolution Method

```rust
impl NameResolutionManager {
    /// Resolve a name to addresses
    pub async fn resolve(&self, name: &str) -> Result<NameResolutionResult> {
        // Validate the name format
        self.validate_name(name)?;
        
        // Check the cache first
        if let Some(cached) = self.check_cache(name).await {
            debug!("Name resolution cache hit for {}", name);
            return Ok(cached);
        }
        
        // Try to resolve via DHT
        match self.resolve_via_dht(name).await {
            Ok(result) => {
                // Cache the result
                self.cache_result(name, &result).await;
                return Ok(result);
            }
            Err(e) => {
                debug!("DHT resolution failed for {}: {}", name, e);
                
                // Try blockchain fallback if enabled
                if self.config.enable_blockchain_fallback {
                    if let Some(blockchain) = &self.blockchain {
                        match blockchain.resolve_name(name).await {
                            Ok(result) => {
                                // Cache the blockchain result
                                self.cache_result(name, &result).await;
                                return Ok(result);
                            }
                            Err(e) => {
                                debug!("Blockchain resolution failed for {}: {}", name, e);
                            }
                        }
                    }
                }
                
                // Return the original DHT error if blockchain fallback failed or disabled
                return Err(e);
            }
        }
    }
    
    /// Resolve a name via DHT
    async fn resolve_via_dht(&self, name: &str) -> Result<NameResolutionResult> {
        // Parse the name components
        let components: Vec<&str> = name.split('.').collect();
        if components.len() != 3 || components[2] != "icn" {
            return Err(Error::invalid_input("Invalid name format"));
        }
        
        let service_name = components[0];
        let coop_id = components[1];
        
        // Construct the DHT key
        let key = format!("{}{}.{}", self.config.dht_prefix, service_name, coop_id);
        
        // Query the DHT
        let value = self.dht.get(key.as_bytes().to_vec()).await?;
        
        // Deserialize the result
        let result: NameResolutionResult = serde_json::from_slice(&value)
            .map_err(|e| Error::decode(format!("Failed to deserialize DHT value: {}", e)))?;
        
        Ok(result)
    }
    
    /// Register a name in the DHT
    pub async fn register_name(&self, name: &str, did: &str, addresses: Vec<NameAddress>) -> Result<()> {
        // Validate the name format
        self.validate_name(name)?;
        
        // Create the resolution result
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
            
        let result = NameResolutionResult {
            name: name.to_string(),
            did: Some(did.to_string()),
            addresses,
            expires_at: now + self.config.cache_ttl,
            source: ResolutionSource::Dht,
        };
        
        // Serialize the result
        let serialized = serde_json::to_vec(&result)
            .map_err(|e| Error::encode(format!("Failed to serialize resolution result: {}", e)))?;
        
        // Parse the name components
        let components: Vec<&str> = name.split('.').collect();
        let service_name = components[0];
        let coop_id = components[1];
        
        // Construct the DHT key
        let key = format!("{}{}.{}", self.config.dht_prefix, service_name, coop_id);
        
        // Store in DHT
        self.dht.put(key.as_bytes().to_vec(), serialized).await?;
        
        // Cache the result locally
        self.cache_result(name, &result).await;
        
        Ok(())
    }
    
    // Other helper methods (validate_name, check_cache, cache_result, etc.)
}
```

## Service Discovery

The name resolution system enables service discovery by allowing services to register themselves in the DHT.

### Service Registration

```rust
impl ServiceRegistry {
    /// Register a service
    pub async fn register_service(&self, service: &Service) -> Result<()> {
        // Validate the service
        self.validate_service(service)?;
        
        // Generate a name for the service
        let name = format!("{}.{}.icn", service.name, service.coop_id);
        
        // Convert service endpoints to name addresses
        let addresses = service.endpoints.iter()
            .map(|endpoint| endpoint.to_name_address())
            .collect::<Vec<_>>();
        
        // Register the name
        self.name_resolver.register_name(&name, &service.did, addresses).await?;
        
        // If blockchain storage is enabled, register there too
        if let Some(blockchain) = &self.blockchain {
            blockchain.register_service(service).await?;
        }
        
        Ok(())
    }
}
```

### Service Lookup

```rust
impl ServiceRegistry {
    /// Look up a service by name
    pub async fn lookup_service(&self, name: &str) -> Result<Service> {
        // Resolve the name
        let resolution = self.name_resolver.resolve(name).await?;
        
        // If we have a DID, look up the service details from that
        if let Some(did) = &resolution.did {
            // Resolve the DID
            let did_doc = self.did_resolver.resolve(did).await?;
            
            // Find the service in the DID document
            if let Some(service) = did_doc.find_service_by_name(name) {
                return Ok(Service {
                    did: did.clone(),
                    name: service.name.clone(),
                    coop_id: service.coop_id.clone(),
                    endpoints: resolution.addresses.iter()
                        .map(|addr| addr.to_service_endpoint())
                        .collect(),
                    attributes: service.attributes.clone(),
                });
            }
        }
        
        // If no DID or service not found in DID document, construct from resolution
        let components: Vec<&str> = name.split('.').collect();
        if components.len() != 3 {
            return Err(Error::invalid_input("Invalid name format"));
        }
        
        let service_name = components[0];
        let coop_id = components[1];
        
        Ok(Service {
            did: resolution.did.unwrap_or_default(),
            name: service_name.to_string(),
            coop_id: coop_id.to_string(),
            endpoints: resolution.addresses.iter()
                .map(|addr| addr.to_service_endpoint())
                .collect(),
            attributes: HashMap::new(),
        })
    }
}
```

## Caching and Performance

### Local Caching Strategy

The name resolution system employs a multi-level caching strategy:

1. **LRU Cache**: Most recently used names are cached in memory
2. **TTL-based Expiry**: Cache entries expire after a configurable TTL
3. **Negative Caching**: Failed resolutions are also cached (with shorter TTL)

```rust
impl NameResolutionManager {
    /// Check the cache for a name
    async fn check_cache(&self, name: &str) -> Option<NameResolutionResult> {
        let cache_guard = self.cache.read().await;
        
        if let Some(cached) = cache_guard.get(name) {
            // Check if the cache entry has expired
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
                
            if cached.expires_at > now {
                return Some(cached.clone());
            }
        }
        
        None
    }
    
    /// Cache a resolution result
    async fn cache_result(&self, name: &str, result: &NameResolutionResult) {
        let mut cache_guard = self.cache.write().await;
        cache_guard.put(name.to_string(), result.clone());
    }
}
```

### Preloading Frequently Used Names

For performance optimization, frequently used names can be preloaded at startup:

```rust
impl NameResolutionManager {
    /// Preload frequently used names
    pub async fn preload_common_names(&self, names: &[String]) -> Result<()> {
        for name in names {
            match self.resolve(name).await {
                Ok(result) => {
                    debug!("Preloaded name {}", name);
                }
                Err(e) => {
                    warn!("Failed to preload name {}: {}", name, e);
                }
            }
        }
        
        Ok(())
    }
}
```

## Security Considerations

### Record Authentication

DHT records are signed to ensure authenticity:

```rust
impl NameResolutionManager {
    /// Verify the authenticity of a DHT record
    async fn verify_dht_record(&self, name: &str, record: &NameResolutionResult) -> Result<bool> {
        // If the record has a DID
        if let Some(did) = &record.did {
            // Resolve the DID to verify the record was created by the DID owner
            let did_doc = self.did_resolver.resolve(did).await?;
            
            // Check that the name follows the expected pattern for this DID
            let components: Vec<&str> = name.split('.').collect();
            if components.len() == 3 {
                let service_name = components[0];
                let coop_id = components[1];
                
                // Extract the cooperative ID from the DID
                let did_parts: Vec<&str> = did.split(':').collect();
                if did_parts.len() >= 3 && did_parts[0] == "did" && did_parts[1] == "icn" {
                    let did_coop_id = did_parts[2];
                    
                    // Verify the cooperative ID matches
                    if coop_id != did_coop_id {
                        warn!("Cooperative ID mismatch: {} vs {}", coop_id, did_coop_id);
                        return Ok(false);
                    }
                    
                    // Specific validation logic omitted for brevity
                    // (should check signatures, timestamps, etc.)
                    
                    return Ok(true);
                }
            }
        }
        
        Ok(false)
    }
}
```

### Preventing Malicious Records

To prevent malicious records:

1. **Cooperative-scoped Names**: Names are scoped to cooperatives
2. **Signature Verification**: DHT records are signed and verified
3. **Blockchain Validation**: Critical records are validated against blockchain
4. **Federation Approval**: Cross-cooperative services require federation approval

## Usage Examples

### 1. Resolving a Service

```rust
// Create a name resolver
let resolver = NameResolutionManager::new(
    dht_service.clone(),
    Some(blockchain_client.clone()),
    did_resolver.clone(),
    NameResolutionConfig::default(),
).await?;

// Resolve a service name
let result = resolver.resolve("database.coopA.icn").await?;

// Connect to the service
let address = result.addresses.iter()
    .filter(|addr| addr.address_type == "ipv6")
    .next()
    .ok_or_else(|| Error::not_found("No IPv6 address found"))?;

// Format the address with port
let socket_addr = format!("[{}]:{}", 
    address.address, 
    address.port.unwrap_or(0)
);

// Connect using the resolved address
let connection = TcpStream::connect(socket_addr).await?;
```

### 2. Registering a Service

```rust
// Create service endpoints
let endpoints = vec![
    ServiceEndpoint {
        endpoint_type: "wireguard".to_string(),
        address: "fd00:abcd:1234::1".to_string(),
        port: Some(5432),
        transport: Some("tcp".to_string()),
        priority: 10,
    },
    ServiceEndpoint {
        endpoint_type: "libp2p".to_string(),
        address: "/ip4/192.168.1.1/tcp/9000/p2p/QmYyQSo1c1Ym7orWxLYvCrM2EmxFTANf8wXmmE7DWjhx5N".to_string(),
        port: None,
        transport: None,
        priority: 20,
    },
];

// Create a service
let service = Service {
    did: "did:icn:coopA:node1".to_string(),
    name: "database".to_string(),
    coop_id: "coopA".to_string(),
    endpoints,
    attributes: HashMap::new(),
};

// Register the service
service_registry.register_service(&service).await?;

// The service is now accessible at database.coopA.icn
```

### 3. System-level Integration

ICN name resolution can be integrated with system-level DNS:

```rust
// Set up a DNS resolver that forwards .icn requests to the ICN resolver
impl DnsServer {
    async fn handle_dns_query(&self, query: &DnsQuery) -> Result<DnsResponse> {
        let domain = query.domain();
        
        // Check if this is an ICN domain
        if domain.ends_with(".icn") {
            // Resolve using ICN name resolution
            match self.icn_resolver.resolve(domain).await {
                Ok(result) => {
                    // Convert ICN resolution to DNS response
                    return Ok(self.convert_to_dns_response(query, result));
                }
                Err(_) => {
                    // Return NXDOMAIN for failed ICN resolutions
                    return Ok(DnsResponse::nx_domain(query));
                }
            }
        }
        
        // Forward non-ICN domains to the regular DNS
        self.regular_dns.resolve(query).await
    }
}
```

## Comparison with Traditional DNS

| Feature | Traditional DNS | ICN DHT Resolution |
|---------|-----------------|-------------------|
| **Control** | Centralized authorities | Cooperative-controlled |
| **Resilience** | Depends on root servers | Fully distributed |
| **Latency** | Low (with caching) | Variable (DHT lookups) |
| **Security** | TLS, DNSSEC | Cryptographic signatures |
| **Verification** | Limited (DNSSEC) | Blockchain-backed |
| **Updates** | Minutes to days | Near real-time |
| **Federation** | Limited | Built-in |

## Blockchain Integration

The blockchain serves as an authoritative, tamper-resistant record store:

```rust
impl BlockchainClient {
    /// Register a name on the blockchain
    pub async fn register_name(&self, name: &str, did: &str, addresses: &[NameAddress]) -> Result<()> {
        // Create the transaction payload
        let payload = NameRegistrationPayload {
            name: name.to_string(),
            did: did.to_string(),
            addresses: addresses.to_vec(),
            timestamp: chrono::Utc::now().timestamp() as u64,
        };
        
        // Sign the payload
        let signature = self.sign_payload(&payload)?;
        
        // Build the transaction
        let transaction = Transaction {
            transaction_type: "NameRegistration".to_string(),
            payload: serde_json::to_value(payload)?,
            signature,
            timestamp: chrono::Utc::now().timestamp() as u64,
        };
        
        // Submit the transaction
        self.submit_transaction(transaction).await?;
        
        Ok(())
    }
    
    /// Resolve a name from the blockchain
    pub async fn resolve_name(&self, name: &str) -> Result<NameResolutionResult> {
        // Query the blockchain
        let response = self.query(
            "NameResolution",
            json!({ "name": name }),
        ).await?;
        
        // Parse the response
        let result: NameResolutionResult = serde_json::from_value(response)
            .map_err(|e| Error::decode(format!("Failed to parse blockchain response: {}", e)))?;
        
        Ok(result)
    }
}
```

## Conclusion

The DHT-based name resolution system in ICN provides a decentralized alternative to traditional DNS, with several key advantages:

1. **No single point of failure** or central authority
2. **Cryptographic verification** of records
3. **Integration with DIDs** for service identity
4. **Blockchain backup** for authoritative records
5. **Federation support** for cross-cooperative services

This system allows ICN nodes to discover and connect to services across the network without relying on centralized infrastructure, enhancing both resilience and security. 