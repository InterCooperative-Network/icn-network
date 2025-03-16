# ICN Network - Intercooperative Network

This document provides instructions for setting up and running the ICN network.

## Overview

The ICN Network is a decentralized, peer-to-peer networking solution designed for cooperative resource sharing, governance, and secure transactions. The foundation of the ICN is built on a robust P2P networking layer that provides:

- Decentralized peer discovery and connection management
- End-to-end encrypted communications
- IPv6-first design for modern networking
- NAT traversal with circuit relay
- Reputation-based peer management
- Resource sharing and coordination

## Prerequisites

Before running the ICN network, ensure you have the following installed:

- Rust and Cargo (latest stable version)
- OpenSSL development libraries
- jq (for script processing)
- IP tools (`ip` command for network interfaces)

### Installing Dependencies

On Debian/Ubuntu systems:
```bash
sudo apt update
sudo apt install build-essential pkg-config libssl-dev jq iproute2
```

For Rust installation, use rustup:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

## Network Setup

We provide two methods to bootstrap the network:

1. **Standard Network** - Supports both IPv4 and IPv6
2. **IPv6-focused Network** - Prioritizes IPv6 connectivity

### Environment Setup

Before bootstrapping the network, it's recommended to run the environment setup script to check for required dependencies and create necessary directories:

```bash
# Make sure you're using bash, not PowerShell
bash scripts/setup_environment.sh
```

This script will:
- Create required data directories
- Check for necessary commands (cargo, jq, openssl, ip)
- Verify IPv6 support on your system
- Detect if you're running in WSL
- Make bootstrap scripts executable

### Standard Network Setup

To bootstrap a standard network with 3 nodes:

```bash
# Make sure you're using bash, not PowerShell
bash scripts/bootstrap_network.sh icn-testnet 3

# Start the network
bash ./start_network.sh
```

### IPv6-focused Network Setup

To bootstrap an IPv6-focused network with 3 nodes:

```bash
# Make sure you're using bash, not PowerShell
bash scripts/bootstrap_ipv6_network.sh icn-testnet-ipv6 3

# Start the network
bash ./start_ipv6_network.sh
```

### Running in WSL (Windows Subsystem for Linux)

If you're using WSL, make sure to:

1. Run all commands in a bash shell, not PowerShell
2. Execute scripts with explicit bash command: `bash ./script.sh`
3. Use the updated scripts which store data in local directories
4. Ensure IPv6 is properly enabled in WSL

If PowerShell is your default terminal in VS Code when connected to WSL, you can switch to bash by:
- Opening a new terminal and selecting bash from the dropdown
- Or running `bash` command to start a bash shell

## Network Configuration

The network configuration is stored in YAML format in the `config/` directory:

- `network.yaml` - Standard network configuration
- `network-ipv6.yaml` - IPv6-focused network configuration

You can adjust these configurations to suit your specific needs.

### Key Configuration Options

The network configuration includes:

- **Transport Settings** - Control how nodes communicate
- **Discovery Methods** - Configure peer discovery mechanisms
- **Dual-stack Settings** - Manage IPv4/IPv6 preferences
- **Federation Parameters** - Set up governance and collaboration
- **Economic Parameters** - Configure the mutual credit system

## Node Operations

### Starting a Single Node

To start an individual node:

```bash
cargo run --bin icn-node -- --config config/nodes/node-001.yaml
```

### Command Line Options

The node binary accepts several command line options:

- `--config <PATH>` - Path to node configuration file
- `--network-config <PATH>` - Path to network configuration file
- `--node-id <ID>` - Override the node ID
- `--listen-addr <ADDR>` - Override the listen address
- `--log-level <LEVEL>` - Set log level (debug, info, warn, error)
- `--data-dir <PATH>` - Override the data directory

### Node Monitoring

The nodes expose Prometheus metrics on port 9090 by default. You can configure a Prometheus server to scrape these endpoints.

## Network Architecture

The ICN network is built with a modular architecture:

- **Core** - Fundamental components like storage and cryptography
- **Identity** - Decentralized identity management
- **Network** - P2P communication and routing
- **Governance** - Democratic decision-making
- **Ledger** - Transaction processing and validation
- **Apps** - Application-level services

## Troubleshooting

### Permission Issues

If you encounter "Permission denied" errors when running scripts:

1. Make sure scripts are executable:
   ```bash
   chmod +x scripts/*.sh
   ```

2. Use the updated scripts which store data in local directories instead of `/var/lib/icn`

3. If you need to use system directories, run with sudo:
   ```bash
   sudo bash scripts/bootstrap_network.sh
   ```

### Bootstrap Script Issues

If you encounter issues with the bootstrap scripts:

1. Make sure you're running them with bash:
   ```bash
   bash scripts/bootstrap_network.sh icn-testnet 3
   ```

2. If you see errors related to text replacement or sed/perl commands, try running the environment setup script first:
   ```bash
   bash scripts/setup_environment.sh
   ```

3. The scripts have been updated to use bash parameter expansion instead of sed/perl for better cross-platform compatibility.

### IPv6 Connectivity Issues

If you're having trouble with IPv6 connectivity:

1. Check if your system has IPv6 support enabled:
   ```bash
   ip -6 addr show
   ```

2. For local testing, you can use the loopback address (::1)

3. Make sure your firewall allows IPv6 traffic on the required ports

4. WSL may have limited IPv6 support depending on the version and configuration

### Connection Problems

If nodes can't connect to each other:

1. Check if the listening ports are open:
   ```bash
   ss -tulpn | grep icn-node
   ```

2. Verify that bootstrap peers are correctly configured

3. Check if mDNS discovery is working on your network

### Shell Issues in WSL

If you see errors like "command not recognized" for bash scripts:

1. Make sure you're in a bash shell, not PowerShell
   ```bash
   # Check your current shell
   echo $SHELL
   
   # Switch to bash if needed
   bash
   ```

2. Run scripts with explicit bash:
   ```bash
   bash ./scripts/bootstrap_network.sh
   ```

## Building From Source

To build the project from source:

```bash
# Build all packages
cargo build --release

# Run tests
cargo test

# Build just the node binary
cargo build --release --bin icn-node
```

## Contributing

We welcome contributions to the ICN Network! Please see CONTRIBUTING.md for details on how to contribute.

## License

ICN Network is dual-licensed under either:

- MIT License
- Apache License, Version 2.0

at your option. 