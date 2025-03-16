# ICN IPv6 Overlay Network Testnet

This directory contains tools and scripts for testing the IPv6 overlay network implementation of the ICN (Intercooperative Network) project.

## Components

The testnet includes the following components:

1. **Testnet Node** (`testnet_node.rs`): A standalone executable node that can join the IPv6 overlay network, connect to peers, create tunnels, and exchange messages.

2. **Launch Script** (`launch_testnet.sh`): A shell script to launch a complete testnet with multiple federation bootstrap nodes, regular nodes, and cross-federation bridges.

3. **Monitor** (`monitor.py`): A Python-based terminal UI for monitoring the testnet status, node connectivity, and message exchange.

4. **Scenario Simulator** (`simulate_scenarios.py`): A Python script that can run specific test scenarios to validate the overlay network functionality.

## Prerequisites

- Rust toolchain
- Python 3.6+
- Python packages: `curses` (for monitoring UI)

## Setting Up the Testnet

### Building the Testnet Node

First, build the testnet node executable:

```bash
cargo build --bin testnet_node
```

### Running a Single Node

To run a single node manually:

```bash
cargo run --bin testnet_node -- \
  --node-id=my-node \
  --federation=federation-a \
  --port=9000 \
  --forwarding-policy=ForwardKnown \
  --log-level=info
```

Available command line options:
- `--node-id`: Unique identifier for the node
- `--federation`: Federation ID (empty for no federation)
- `--port`: Port to listen on
- `--bootstrap-peers`: Comma-separated list of overlay addresses to connect to
- `--forwarding-policy`: Packet forwarding policy (ForwardAll, ForwardKnown, NoForwarding)
- `--log-level`: Logging level (trace, debug, info, warn, error)

### Launching a Complete Testnet

To launch a complete testnet with multiple nodes in different federations:

```bash
./testnet/launch_testnet.sh
```

This will start:
- 3 federation bootstrap nodes (A, B, C)
- 8 regular nodes in these federations
- 3 cross-federation bridge nodes
- 1 independent node (not in any federation)

The script will display the assigned overlay addresses for each node and create log files in the `logs` directory.

## Monitoring the Testnet

To monitor the testnet status in real-time:

```bash
python testnet/monitor.py
```

The monitor displays:
- Node status (active, inactive, offline)
- Federation membership
- Connection status
- Message exchange statistics
- Recent messages

Press `q` to quit the monitor.

## Running Test Scenarios

The scenario simulator can run predefined test scenarios to validate different aspects of the overlay network:

```bash
python testnet/simulate_scenarios.py --scenario=basic
```

Available scenarios:
- `basic`: Tests basic connectivity between nodes in different federations
- `isolation`: Tests federation isolation and forwarding policies
- `failure`: Tests network resilience against node failures
- `all`: Runs all scenarios in sequence

## Testnet Architecture

The testnet is organized into federations, each with its own bootstrap node. The federation structure enables testing of:

1. **Federation-based address allocation**: Nodes within a federation get addresses with the same prefix
2. **Routing policies**: Federation-aware packet forwarding
3. **Cross-federation communication**: Via bridge nodes with appropriate forwarding policies

### Federation Structure

- **Federation A**: Main federation with bootstrap node and members
- **Federation B**: Secondary federation with bootstrap node and members
- **Federation C**: Tertiary federation with bootstrap node and members
- **Cross-federation bridges**: Nodes that connect multiple federations
- **Independent nodes**: Nodes not belonging to any federation

## Adding Custom Scenarios

To add a custom test scenario:

1. Edit the `simulate_scenarios.py` file
2. Add a new function following the pattern of existing scenarios
3. Update the `main()` function to include your new scenario in the argument parser

## Troubleshooting

### Common Issues

1. **Nodes can't connect to bootstrap**:
   - Check that the bootstrap node is running
   - Verify the overlay address is correctly entered
   - Ensure network ports are not blocked

2. **Cross-federation routing doesn't work**:
   - Verify bridge nodes have the `ForwardAll` policy
   - Check bridge nodes are successfully connected to both federations

3. **Monitor shows all nodes as "offline"**:
   - Check the log files in the `logs` directory
   - Verify that nodes are actually running

### Viewing Logs

Each node creates a log file in the `logs` directory. To view the logs for a specific node:

```bash
tail -f logs/federation-a-bootstrap.log
```

## Advanced Usage

### Testing with Custom Parameters

You can modify the `testnet/config/testnet.toml` file to adjust various parameters:
- Federation prefix lengths
- Default tunnel types
- Connection timeouts
- Peer discovery intervals

### Custom Node Topologies

To create a custom topology:
1. Create a new launch script based on `launch_testnet.sh`
2. Adjust the node creation and bootstrap peer configuration

## Contributing

To contribute to the testnet:
1. Add new test scenarios that validate specific overlay network functionality
2. Improve the monitoring tools to provide more detailed insights
3. Extend the testnet node with additional features from the main implementation

## License

This testnet implementation is part of the ICN project and follows the same licensing terms. 