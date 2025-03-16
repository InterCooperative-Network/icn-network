# WireGuard Integration Guide

This guide explains how WireGuard is integrated with the ICN network to provide secure overlay networking with dynamic peer configuration.

## Overview

The WireGuard integration in ICN creates an encrypted overlay network where:

1. **Every ICN server is a WireGuard node** 
2. **Traffic is end-to-end encrypted**
3. **Peers auto-negotiate tunnels** without static configuration
4. **Dynamic IPv6 addressing** maps to DIDs

## Architecture

```
┌─────────────────────────────────┐
│                                 │
│           Application           │
│              Layer              │
│                                 │
├─────────────────────────────────┤
│                                 │
│          ICN Network            │
│            Stack                │
│                                 │
├─────────────────────────────────┤
│                                 │
│       WireGuard Interface       │
│                                 │
└─────────────────────────────────┘
              ↑   ↓
┌─────────────────────────────────┐
│                                 │
│         Internet / LAN          │
│                                 │
└─────────────────────────────────┘
```

### Component Relationships

```
┌─────────────────────────────────────────────────────────────────────┐
│                                                                     │
│                         ICN Node                                    │
│                                                                     │
│  ┌───────────────┐                              ┌────────────────┐  │
│  │               │                              │                │  │
│  │ Identity &    │                              │  Application   │  │
│  │ Authentication│                              │  Services      │  │
│  │               │                              │                │  │
│  └───────┬───────┘                              └────────┬───────┘  │
│          │                                               │          │
│          ▼                                               ▼          │
│  ┌───────────────┐     ┌───────────────┐      ┌────────────────┐   │
│  │               │     │               │      │                │   │
│  │ WireGuard     │◄───►│ DHT-based     │◄────►│  Service       │   │
│  │ Manager       │     │ Key Exchange  │      │  Registry      │   │
│  │               │     │               │      │                │   │
│  └───────┬───────┘     └───────────────┘      └────────────────┘   │
│          │                                                          │
│          ▼                                                          │
│  ┌───────────────┐                                                  │
│  │               │                                                  │
│  │ WireGuard     │                                                  │
│  │ Interface     │                                                  │
│  │               │                                                  │
│  └───────┬───────┘                                                  │
│          │                                                          │
└──────────┼──────────────────────────────────────────────────────────┘
           │
           ▼
    Encrypted Tunnel
           │
           ▼
┌─────────────────────┐
│                     │
│   Other ICN Nodes   │
│                     │
└─────────────────────┘
```

## Key Components

### 1. WireGuard Manager

The WireGuard Manager component is responsible for:

- Generating and storing WireGuard keypairs
- Configuring the WireGuard interface
- Mapping DIDs to WireGuard public keys
- Assigning and managing IPv6 addresses
- Dynamically adding/removing peers

```rust
/// Configuration for WireGuard
#[derive(Clone, Debug)]
pub struct WireguardConfig {
    /// Interface name (default: icn0)
    pub interface_name: String,
    
    /// IPv6 prefix for the overlay network
    pub ipv6_prefix: String,
    
    /// Listen port
    pub listen_port: Option<u16>,
    
    /// MTU
    pub mtu: Option<u32>,
    
    /// Persistent keepalive interval
    pub persistent_keepalive: u16,
    
    /// Enable fwmark
    pub fwmark: Option<u32>,
}

/// WireGuard manager
pub struct WireguardManager {
    /// Configuration
    config: WireguardConfig,
    
    /// Storage
    storage: Arc<dyn Storage>,
    
    /// DHT service
    dht: Arc<DhtService>,
    
    /// WireGuard device
    device: WgDevice,
    
    /// Interface name
    interface_name: String,
    
    /// WireGuard keypair
    keypair: WgKeypair,
    
    /// IPv6 network prefix
    ipv6_prefix: Ipv6Net,
    
    /// Peer configurations
    peer_configs: Arc<RwLock<HashMap<String, WireguardPeerConfig>>>,
    
    /// Running state
    running: Arc<RwLock<bool>>,
    
    /// DID resolver
    did_resolver: Arc<DidResolver>,
}
```

### 2. DHT-based Key Exchange

The Distributed Hash Table (DHT) is used to store and exchange WireGuard public keys:

- WireGuard public keys are stored in DID Documents
- Nodes query the DHT to find peer public keys
- Key exchange happens without direct communication
- Updates are propagated through the DHT

### 3. IPv6 Overlay Network

ICN creates an IPv6 overlay network where:

- Each node gets a unique IPv6 address from a private range
- IPv6 addresses are deterministically derived from DIDs
- The overlay network enables direct communication between any two nodes
- All traffic is encrypted using WireGuard

## Implementation Details

### WireGuard Interface Setup

When an ICN node starts, it initializes the WireGuard interface:

```rust
impl WireguardManager {
    /// Initialize the WireGuard interface
    async fn initialize_interface(&self) -> Result<()> {
        // Generate or load keypair
        let keypair = self.load_or_create_keypair().await?;
        
        // Calculate our IPv6 address from DID
        let our_ipv6 = self.calculate_ipv6_for_did(&self.local_did).await?;
        
        // Configure the WireGuard interface
        self.device.set_interface(
            &self.interface_name,
            &keypair.private_key_string(),
            self.config.listen_port,
            self.config.fwmark,
            self.config.mtu,
        )?;
        
        // Add IPv6 address to interface
        self.device.add_address(&self.interface_name, &format!("{}/128", our_ipv6))?;
        
        // Set up routing for the overlay network
        self.device.add_route(&self.interface_name, &format!("{}", self.ipv6_prefix))?;
        
        // Bring up the interface
        self.device.set_interface_up(&self.interface_name)?;
        
        Ok(())
    }
}
```

### WireGuard Key Storage in DID Documents

The WireGuard public key is stored in the DID Document as a service endpoint:

```json
{
  "id": "did:icn:coopA:nodeX",
  "verificationMethod": [...],
  "authentication": [...],
  "service": [
    {
      "id": "did:icn:coopA:nodeX#wireguard",
      "type": "WireGuardEndpoint",
      "serviceEndpoint": {
        "publicKey": "kXr4/JVeJD8pXjPRpwVsmlVnW8kD9/rv+AcOIk5su3A=",
        "ipv6Address": "fd00:abcd:1234::1"
      }
    }
  ]
}
```

### Dynamic Peer Configuration

When a connection to a peer is requested, the WireGuard peer is configured dynamically:

```rust
impl WireguardManager {
    /// Configure a peer tunnel
    pub async fn configure_peer(&self, peer_did: &str) -> Result<()> {
        // Resolve the peer's DID to get WireGuard information
        let did_doc = self.did_resolver.resolve(peer_did).await?;
        
        // Find the WireGuard service in DID Document
        let wg_service = did_doc.find_service("WireGuardEndpoint")
            .ok_or_else(|| Error::not_found("WireGuard service not found in DID Document"))?;
        
        // Get the peer's WireGuard public key
        let public_key = wg_service.extract_wireguard_public_key()?;
        
        // Get peer's IPv6 address 
        let ipv6_address = wg_service.extract_ipv6_address()?;
        
        // Add the peer to WireGuard
        self.device.add_peer(
            &self.interface_name,
            &public_key,
            None, // No endpoint means we're using a peer-to-peer mesh
            &[format!("{}/128", ipv6_address)],
            self.config.persistent_keepalive,
        )?;
        
        // Store peer configuration
        self.peer_configs.write().await.insert(
            peer_did.to_string(),
            WireguardPeerConfig {
                public_key,
                ipv6_address: ipv6_address.parse()?,
                last_handshake: None,
            },
        );
        
        // Log the new connection
        info!("Configured WireGuard tunnel to peer {}", peer_did);
        
        Ok(())
    }
}
```

### IPv6 Address Allocation

IPv6 addresses are deterministically derived from DIDs:

```rust
impl WireguardManager {
    /// Calculate IPv6 address for a DID
    async fn calculate_ipv6_for_did(&self, did: &str) -> Result<IpAddr> {
        // Parse the DID to extract components
        let did_parts = did.split(':').collect::<Vec<_>>();
        if did_parts.len() < 4 || did_parts[0] != "did" || did_parts[1] != "icn" {
            return Err(Error::invalid_input("Invalid DID format"));
        }
        
        // Extract coop_id and node_id
        let coop_id = did_parts[2];
        let node_id = did_parts[3];
        
        // Hash the DID to create a deterministic address
        let hash = sha256(did.as_bytes());
        let addr_bytes = &hash[0..16]; // Use first 16 bytes for IPv6
        
        // Create IPv6 address with network prefix
        let mut ip_bytes = [0u8; 16];
        
        // Copy network prefix (first 8 bytes)
        let prefix_bytes = self.ipv6_prefix.network().octets();
        ip_bytes[0..8].copy_from_slice(&prefix_bytes[0..8]);
        
        // Use the hash for the host part (last 8 bytes)
        ip_bytes[8..16].copy_from_slice(&addr_bytes[8..16]);
        
        Ok(IpAddr::V6(Ipv6Addr::from(ip_bytes)))
    }
}
```

## Usage Examples

### 1. Setting Up an ICN Node with WireGuard

```rust
// Initialize the ICN node with WireGuard
let wireugard_config = WireguardConfig {
    interface_name: "icn0".to_string(),
    ipv6_prefix: "fd00:abcd::/64".to_string(),
    listen_port: Some(51820),
    mtu: Some(1420),
    persistent_keepalive: 25,
    fwmark: None,
};

let manager = WireguardManager::new(
    storage.clone(),
    dht_service.clone(),
    did_resolver.clone(),
    wireugard_config,
).await?;

// Start the WireGuard manager
manager.start().await?;
```

### 2. Connecting to a Peer

```rust
// Connect to a peer using their DID
let peer_did = "did:icn:coopB:node1";
manager.connect_to_peer(peer_did).await?;

// Now you can communicate with the peer using their IPv6 address
let peer_ipv6 = manager.get_peer_ipv6(peer_did).await?;
println!("Connected to peer at {}", peer_ipv6);
```

### 3. Using the Overlay Network

```rust
// Example: Access a service on the overlay network
let service_record = service_registry.lookup_service("database.coopB.icn").await?;

// Get the WireGuard endpoint from the service record
let wg_endpoint = service_record.find_endpoint("wg")?;

// Connect directly using the IPv6 address
let socket = TcpStream::connect((wg_endpoint.address, wg_endpoint.port)).await?;
```

## Security Considerations

### 1. Key Management

- WireGuard keypairs are generated securely using strong randomness
- Private keys never leave the node
- Public keys are distributed through the authenticated DHT
- Keys can be rotated periodically for enhanced security

### 2. Peer Authentication

Before establishing a WireGuard tunnel:

1. The DID is resolved and verified
2. The peer's DID Document is authenticated
3. Signature verification ensures the WireGuard key is legitimate
4. Only after verification is the tunnel established

### 3. Traffic Isolation

- Each cooperative can use a different IPv6 prefix for isolation
- Access control lists can restrict traffic between specific peers
- Federation boundaries enforce additional traffic policies

## Troubleshooting

### Common Issues

1. **Tunnel Not Establishing**
   - Check that both peers have properly published their WireGuard keys in their DID Documents
   - Verify that UDP port 51820 (or your configured port) is open in firewalls
   - Check that the WireGuard interface is up and running

2. **Cannot Resolve DIDs**
   - Ensure the DHT service is operational
   - Verify that bootstrap nodes are correctly configured
   - Check network connectivity to the DHT network

3. **IPv6 Routing Problems**
   - Verify the local IPv6 configuration
   - Ensure the routing table includes the overlay network
   - Check if any firewall is blocking IPv6 traffic

### Diagnostic Commands

```bash
# Check WireGuard interface status
$ sudo wg show icn0

# View IPv6 addresses
$ ip -6 addr show dev icn0

# Check routing table
$ ip -6 route show

# Test connectivity to a peer
$ ping6 fd00:abcd:1234::1
```

## Performance Tuning

For optimal WireGuard performance in the ICN network:

1. **MTU Optimization**
   - Set the MTU to account for WireGuard overhead (usually 1420)
   - Adjust based on your network conditions

2. **Keepalive Intervals**
   - Default is 25 seconds
   - Decrease for more responsive connections
   - Increase to reduce bandwidth usage

3. **Handshake Timeout**
   - Configure handshake timeout based on network reliability
   - Lower values detect offline peers faster

## Conclusion

The WireGuard integration in ICN provides:

1. **Secure overlay networking** with end-to-end encryption
2. **Dynamic peer configuration** without manual setup
3. **Deterministic IPv6 addressing** based on DIDs
4. **Seamless integration** with the DID system

This enables a fully encrypted mesh network where nodes can securely communicate regardless of their physical network topology or firewall restrictions. 