# ICN Federation-Aware Networking

This module implements a federation-aware networking layer for the ICN (Intercooperative Network) system. 
The networking module enables cooperatives to form network federations with specific security 
boundaries, resource sharing policies, and communication patterns.

## Architecture

The network module is built on several key components:

1. **NetworkManager**: Manages network connections, federation membership, and secure communications
2. **WireGuardOverlay**: Provides secure tunneling between peers using WireGuard
3. **Federation System**: Allows multiple cooperatives to form federation networks with shared governance
4. **Federation Governance Integration**: Connects networking with democratic governance

## Federation Features

A federation in ICN represents a logical grouping of peers that share:

- Common governance rules
- Security boundaries
- DHT namespace for resource discovery
- Encryption standards
- WireGuard overlay (optional)

### Multi-Federation Support

The system allows nodes to participate in multiple federations simultaneously, with:

- Federation-specific configurations
- Isolated DHT namespaces
- Federation-aware messaging
- Cross-federation communication controls
- Federation-specific metrics and monitoring

## Federation Governance

The federation governance integration allows cooperative members to:

- Democratically decide on network configuration changes
- Vote on adding or removing peers
- Control cross-federation communication policies
- Enable or disable security features like WireGuard
- Manage bootstrap peers and federation connectivity

Governance is synchronized across the federation so that all nodes can participate in the decision-making process.
Proposals, votes, and execution results are shared via the federation messaging system.

### Governance Workflow

1. A member creates a proposal for a network change
2. Members vote on the proposal (yes, no, abstain)
3. When quorum is reached and approval threshold is met, the proposal is approved
4. The approved proposal can be executed, which applies the changes to the network
5. Execution results are broadcast to all federation peers

## CLI Commands

The networking module exposes the following federation-related commands through the CLI:

### Federation Management

- `network create-federation`: Create a new federation
- `network list-federations`: List available federations
- `network switch-federation`: Switch the active federation
- `network federation-info`: Show information about a federation
- `network federation-peers`: List peers in a federation
- `network federation-metrics`: Show federation metrics

### Federation Communication

- `network broadcast-to-federation`: Send a message to all federation peers
- `network enable-federation-wireguard`: Enable WireGuard for a federation

### Federation Governance

- `network governance create-proposal`: Create a network governance proposal
- `network governance list-proposals`: List network governance proposals
- `network governance show-proposal`: Show details of a specific proposal
- `network governance vote`: Cast a vote on a network proposal
- `network governance execute-proposal`: Execute an approved proposal
- `network governance sync-governance`: Sync governance data with federation

## Security Considerations

Federation-aware networking provides several security enhancements:

1. **Isolation**: Each federation has its own DHT namespace and communication channels
2. **Access Control**: Federation membership can be controlled via governance mechanisms
3. **Cross-Federation Boundaries**: Explicit control over which federations can communicate
4. **Encryption**: Federation-specific encryption settings
5. **WireGuard Integration**: Optional secure overlay networks for each federation
6. **Democratic Control**: Network changes require consensus through governance

## Future Enhancements

The federation-aware networking system will be expanded with:

1. **Federation Governance Integration**: Link federation membership to governance decisions
2. **Resource Discovery**: Federation-aware resource publishing and discovery
3. **Federation Metrics**: Enhanced metrics for federation health and activity
4. **Automated Federation Membership**: Dynamic federation joining based on credentials
5. **Federation Network Sharding**: Performance optimization for large federations
6. **Role-Based Access Control**: Granular permissions within federation governance

## Usage Examples

### Create a Federation

```bash
icn-cli network create-federation -i myfederation -b "peer1,peer2,peer3" --allow-cross-federation
```

### Switch Between Federations

```bash
icn-cli network switch-federation -i otherfederation
```

### Create a Network Governance Proposal

```bash
icn-cli network governance create-proposal -t "Add Bootstrap Peer" -d "Add a new bootstrap peer to improve network reliability" -p member1 -p add-bootstrap -p '{"peers": ["peer1.example.com", "peer2.example.com"]}'
```

### Vote on a Proposal

```bash
icn-cli network governance vote -i proposal123 -m member2 -v yes -c "This will improve network stability"
```

### Execute an Approved Proposal

```bash
icn-cli network governance execute-proposal -i proposal123
``` 