# ICN Network

A comprehensive peer-to-peer networking solution for distributed applications, with a focus on performance, security, and reliability.

## Features

- **Metrics and Monitoring**: Comprehensive metrics collection for network performance and behavior tracking
- **Peer Reputation System**: Intelligent tracking of peer behavior with automatic banning of misbehaving nodes
- **Priority Message Processing**: Smart message queue that prioritizes important messages and high-reputation peers
- **Circuit Relay**: NAT traversal for connecting nodes behind firewalls
- **Peer Discovery**: Multiple discovery mechanisms to find peers (mDNS, Kademlia DHT, bootstrap peers)
- **Message Exchange**: Efficient gossip-based message propagation

## Project Structure

- `crates/network/` - Main network crate
  - `src/` - Source code
  - `examples/` - Example applications
  - `scripts/` - Utility scripts
  - `tests/` - Integration tests

## Getting Started

### Prerequisites

- Rust 1.70 or later
- Cargo
- (For running tests) curl and jq

### Building the Project

```bash
cargo build
```

### Running Examples

```bash
# Run a specific example
cargo run --example metrics_demo

# Or use the demo script to explore all examples
./crates/network/scripts/run_demos.sh
```

## Testing

### Running Unit Tests

```bash
cargo test
```

### Running Integration Tests

```bash
./crates/network/scripts/test_all_features.sh
```

This script will test all the major features:
1. Metrics and monitoring
2. Peer reputation system
3. Priority message processing
4. Circuit relay for NAT traversal
5. Integrated demo with all features

## Demo Applications

The project includes several demos:

- **Metrics Demo**: Demonstrates real-time network metrics collection
- **Reputation Demo**: Shows how the peer reputation system works
- **Priority Messaging Demo**: Demonstrates priority-based message processing
- **Circuit Relay Demo**: Shows NAT traversal using relay servers
- **Integrated Demo**: Combines all features into a single application

Run any demo using:

```bash
cargo run --example <demo_name>
```

## Documentation

For more detailed documentation, see:

- [Network Crate Documentation](crates/network/README.md) - Details on the network crate
- API Documentation (generate with `cargo doc --open`)

## License

This project is dual-licensed under:
- MIT License
- Apache License, Version 2.0
