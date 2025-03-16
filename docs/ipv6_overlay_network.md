# IPv6 Overlay Network

This document describes the IPv6-based overlay network implementation for the ICN (Intercooperative Network) project.

## Overview

The ICN overlay network uses IPv6 as its addressing and routing protocol, creating a decentralized, peer-to-peer mesh network that can span across multiple federations. It provides secure tunneling options for communication between nodes, including direct IPv6, WireGuard, TLS, and onion-routed connections.

## Architecture

The overlay network consists of the following components:

1. **Address Management** - Allocates and manages IPv6-like addresses in the overlay network.
2. **Routing** - Manages routes between nodes in the overlay network.
3. **Tunneling** - Provides secure tunneling between nodes.
4. **DHT (Distributed Hash Table)** - Enables discovery and distributed data storage.
5. **Onion Routing** - Supports privacy-enhancing routing for anonymous communication.

## IPv6 Address Allocation

The overlay network uses IPv6 addressing with the following schemes:

- **Unique Local Addresses (ULA)** - Uses the `fd00::/8` IPv6 prefix for private network addressing.
- **Global Unicast Addresses (GUA)** - Uses `2001::/16` prefix for public network addressing (in a real production environment).
- **Federation Prefixing** - Federations have their own subnet prefixes (typically /48).
- **Node Addressing** - Individual nodes receive /64 subnets.

Addresses are allocated using one of several strategies:

- **Random** - Generates random addresses within the specified space.
- **NodeIdBased** - Derives addresses from the node ID using a hash function.
- **FederationPrefixed** - Combines federation prefixes with node-specific identifiers.
- **GeographicBased** - Allocates addresses based on geographic location (future enhancement).

## Tunneling

The overlay network supports different types of tunnels for secure communication:

1. **Direct IPv6** - Direct communication between nodes that can reach each other via IPv6.
2. **WireGuard** - Uses WireGuard for secure, high-performance tunneling.
3. **TLS** - Provides SSL/TLS encrypted tunnels for secure communication.
4. **Onion** - Uses onion routing for enhanced privacy and anonymity.

Tunnels are automatically selected based on the relationship between nodes:

- Nodes in the same federation typically use direct connections.
- Nodes in different federations use WireGuard tunnels by default.
- When anonymity is required, onion tunnels are used.

## Routing

Routing in the overlay network uses a combination of:

- **Direct Routing** - For nodes that can directly communicate.
- **Federation Routing** - Using federation gateways for cross-federation communication.
- **DHT-based Routing** - For discovering paths to unknown nodes.

Routes include information about:
- Destination address
- Next hop address (if any)
- Complete path to the destination
- Cost metric
- Last update timestamp

## Packet Forwarding

Nodes can act as routers, forwarding packets according to one of three policies:

1. **ForwardAll** - Forwards all packets regardless of destination.
2. **ForwardKnown** - Only forwards packets to known destinations.
3. **NoForwarding** - Does not forward any packets.

## IPv6 Packet Structure

The overlay network uses a custom IPv6 packet format with the following fields:

- Source address (OverlayAddress)
- Destination address (OverlayAddress)
- Next header (protocol)
- Hop limit (TTL)
- Traffic class (for QoS)
- Flow label
- Payload data

## Federation Support

Federations in the overlay network are logical groupings of nodes that share:

- Common address prefixes
- Security policies
- Resource sharing permissions

Communication between federations is secured using WireGuard tunnels by default.

## Implementation Details

### Address Management

The `AddressAllocator` handles the allocation of addresses based on node IDs and federation IDs. It ensures that:

- Addresses are unique within the network
- Federation prefixes are consistently applied
- Addresses follow IPv6 standards

### Tunneling

The `TunnelManager` provides:

- Creation and management of tunnels
- Monitoring of tunnel health
- Automatic selection of tunnel types
- Statistics gathering on tunnel performance

### Routing

The `RouteManager` provides:

- Route discovery and management
- Federation-based routing
- Cost-based route selection
- Route health monitoring

## Example Use

```rust
// Create an address allocator
let mut address_allocator = AddressAllocator::with_settings(
    AddressSpace::UniqueLocal,
    AddressAllocationStrategy::FederationPrefixed,
    48,  // Federation prefix length
    64   // Node prefix length
);

// Create and initialize a node
let mut node = OverlayNetworkManager::with_address_allocator(address_allocator);
let addr = node.initialize("node1", Some("federation-alpha")).await?;

// Connect to bootstrap nodes
node.connect(&[bootstrap_addr]).await?;

// Create a tunnel to another node
let tunnel = node.create_tunnel(&remote_addr, TunnelType::WireGuard).await?;

// Send data through the overlay
let options = OverlayOptions {
    anonymity_required: false,
    reliability_required: true,
    priority: MessagePriority::Normal,
    tunnel_type: Some(TunnelType::WireGuard),
    ttl: 64,
};
node.send_data(&destination_addr, data, &options).await?;
```

## Security Considerations

The overlay network implements several security measures:

1. **Encryption** - All tunnel types except Direct provide encryption.
2. **Authentication** - Nodes authenticate peers before establishing connections.
3. **Privacy** - Onion routing provides enhanced privacy when needed.
4. **Access Control** - Federation-based access control limits which nodes can communicate.

## Future Enhancements

Planned enhancements to the overlay network include:

1. **NAT Traversal** - Enhanced techniques for traversing NATs and firewalls.
2. **Multi-path Routing** - Using multiple paths for increased reliability and throughput.
3. **Quality of Service** - Enhanced QoS based on traffic class and priority.
4. **Cross-Federation Governance** - Policy-based routing and access control between federations.
5. **IPv6 Prefix Delegation** - Dynamic address allocation for hierarchical networks.
6. **Mobile Node Support** - Better handling of nodes that change their network attachment point. 