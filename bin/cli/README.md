# ICN Command Line Interface

A command-line tool for interacting with the Intercooperative Network (ICN) services.

## Features

- Network status and connectivity testing
- Distributed storage system with federation support
- End-to-end encryption of stored data
- Version history tracking for stored files
- Multi-federation file management
- Decentralized governance system with proposal management and voting

## Installation

Build from source:

```bash
cargo build --release -p icn-cli
```

The executable will be available at `target/release/icn-cli`.

## Usage

### Show CLI help and available commands

```bash
icn-cli --help
```

### Enable verbose output for debugging

```bash
icn-cli -v <command>  # Increase verbosity level
icn-cli -vv <command> # Very verbose output for debugging
```

## Storage System Commands

The ICN CLI includes a distributed storage system with support for:
- End-to-end encryption
- Data versioning
- Multi-federation storage

### Initialize Storage Environment

```bash
# Initialize with default settings
icn-cli storage init

# Initialize with a custom path and encryption enabled
icn-cli storage init --path /path/to/storage --encrypted
```

### Generate Encryption Key

Before storing encrypted files, generate an encryption key:

```bash
icn-cli storage generate-key
```

### Store Files

```bash
# Store a file with auto-generated key (filename)
icn-cli storage put --file document.pdf

# Store with custom key and encryption
icn-cli storage put --file large_file.zip --key important_data --encrypted

# Store in a specific federation
icn-cli storage put --file config.json --federation europe
```

### Retrieve Files

```bash
# Retrieve the latest version
icn-cli storage get --key document.pdf

# Retrieve a specific version
icn-cli storage get --key important_data --version 83f67d4a-e89b-4e2c-9f15-2a5850356121

# Retrieve to a specific location
icn-cli storage get --key config.json --output /tmp/restored-config.json
```

### List Files

```bash
# List all files
icn-cli storage list

# List files with a specific prefix
icn-cli storage list --prefix backup_

# List files in a specific federation
icn-cli storage list --federation europe
```

### View Version History

```bash
# Show version history for a file
icn-cli storage history --key document.pdf

# Show limited history
icn-cli storage history --key document.pdf --limit 5
```

## Governance Commands

The ICN CLI includes a decentralized governance system that enables democratic decision-making within federations:

### Create Proposals

```bash
# Create a simple policy change proposal
icn-cli governance create-proposal \
  --title "Update Network Policy" \
  --description "Update the network policy to increase security standards" \
  --proposal-type policy \
  --proposer john.doe

# Create a proposal with custom content from a JSON file
icn-cli governance create-proposal \
  --title "Add New Member" \
  --description "Add XYZ Cooperative as a new member" \
  --proposal-type member-add \
  --proposer admin \
  --content-file member-details.json \
  --quorum 75 \
  --approval 67
```

### Manage Proposals

```bash
# List all proposals
icn-cli governance list-proposals

# List proposals by status
icn-cli governance list-proposals --status voting

# Show detailed proposal information
icn-cli governance show-proposal --id 83f67d4a-e89b-4e2c-9f15-2a5850356121

# Update proposal status
icn-cli governance update-status --id 83f67d4a-e89b-4e2c-9f15-2a5850356121 --status deliberation
```

### Voting Process

```bash
# Start voting period for a proposal
icn-cli governance start-voting --id 83f67d4a-e89b-4e2c-9f15-2a5850356121 --duration 604800

# Cast a vote
icn-cli governance vote --id 83f67d4a-e89b-4e2c-9f15-2a5850356121 --member alice --vote yes --weight 2.5
icn-cli governance vote --id 83f67d4a-e89b-4e2c-9f15-2a5850356121 --member bob --vote no --comment "Needs more discussion"

# Finalize voting
icn-cli governance finalize-voting --id 83f67d4a-e89b-4e2c-9f15-2a5850356121

# Execute approved proposal
icn-cli governance execute-proposal --id 83f67d4a-e89b-4e2c-9f15-2a5850356121
```

## Network Commands

### Check Node Status

```bash
icn-cli status
```

### Test Network Connectivity

```bash
# Test with default server
icn-cli network

# Test with specific server
icn-cli network --server 192.168.1.100:8080
```

## Development

To run the CLI in development mode:

```bash
cargo run -p icn-cli -- <command>
```

## License

MIT OR Apache-2.0 