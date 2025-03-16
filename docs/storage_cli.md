# ICN Storage CLI

The ICN Storage CLI provides a command-line interface for interacting with the ICN Network distributed storage system.

## Installation

```bash
# Build from source
cargo build --bin storage-cli

# Run the CLI
cargo run --bin storage-cli -- <command> [options]
```

## Commands Overview

The CLI offers the following main command categories:

- `init`: Initialize the storage environment
- `federation`: Manage federations
- `storage`: Manage storage operations
- `version`: Manage data versions
- `encryption`: Manage encryption
- `metrics`: View and manage metrics
- `status`: Show storage system status

## Initialize Storage

Before using the storage system, you need to initialize it:

```bash
# Initialize storage with default settings
storage-cli init

# Initialize with custom settings
storage-cli init --data-dir /path/to/data --node-id mynode --address 192.168.1.10:8000 --capacity 2147483648
```

This creates a local configuration file and prepares the storage directory.

## Managing Federations

Federations are collaborative groups that share resources in the ICN Network:

```bash
# Create a new federation
storage-cli federation create "My Federation" --description "A federation for testing"

# List available federations
storage-cli federation list

# Join an existing federation
storage-cli federation join fed-123456
```

## Managing Storage

Store, retrieve, and delete data:

```bash
# Store data from a file with versioning and encryption enabled
storage-cli storage put my/key/path --file /path/to/file.txt --versioned --encrypted

# Store data from a string
storage-cli storage put my/key/path --data "Hello, world!"

# Store data from stdin
cat file.txt | storage-cli storage put my/key/path

# Retrieve data
storage-cli storage get my/key/path

# Save retrieved data to a file
storage-cli storage get my/key/path --output /path/to/output.txt

# Delete data
storage-cli storage delete my/key/path
```

### Advanced Storage Options

```bash
# Store with specific federation access
storage-cli storage put my/key/path --file file.txt --federation fed1 --federation fed2

# Set redundancy factor (number of replicas)
storage-cli storage put my/key/path --file file.txt --redundancy 5

# Limit number of versions to keep
storage-cli storage put my/key/path --file file.txt --versioned --max-versions 20
```

## Managing Versions

Work with versioned data:

```bash
# List all versions for a key
storage-cli version list my/key/path

# Get a specific version by ID
storage-cli version get my/key/path v-1234567890

# Save a specific version to a file
storage-cli version get my/key/path v-1234567890 --output /path/to/output.txt

# Revert to a previous version
storage-cli version revert my/key/path v-1234567890

# Enable versioning for existing data
storage-cli version enable my/key/path --max-versions 10
```

## Managing Encryption

Work with encryption:

```bash
# Create a new encryption key for specific federations
storage-cli encryption create-key fed1 fed2 fed3

# Grant a federation access to an existing key
storage-cli encryption grant-access fed4 key-1234567890
```

## System Status

View the current status of the storage system:

```bash
# Show system status
storage-cli status
```

This displays information about your node, federations, storage peers, and usage statistics.

## Monitoring and Metrics

The CLI provides built-in metrics collection and reporting capabilities:

```bash
# Show current metrics in human-readable format
storage-cli metrics show

# Show metrics in JSON format
storage-cli metrics show --format json

# Reset all metrics counters
storage-cli metrics reset

# Export metrics to a JSON file
storage-cli metrics export metrics.json

# Export metrics to a CSV file for analysis
storage-cli metrics export metrics.csv --format csv
```

The metrics system tracks:

- Operation counts (puts, gets, deletes, version operations)
- Operation latencies
- Data statistics (keys, size, encryption, versioning)
- Version statistics (counts, sizes, overhead)

This data helps you monitor performance, identify bottlenecks, and understand storage usage patterns over time.

## Examples

### Storing and retrieving a versioned document

```bash
# Initialize storage
storage-cli init

# Create a federation
storage-cli federation create "My Federation" --description "Test Federation"

# Store a document with versioning enabled
echo "Version 1 content" > doc.txt
storage-cli storage put docs/example.txt --file doc.txt --versioned

# Update the document
echo "Version 2 content" > doc.txt
storage-cli storage put docs/example.txt --file doc.txt

# List versions
storage-cli version list docs/example.txt

# Revert to the first version
storage-cli version revert docs/example.txt v-<version-id-from-list>

# Verify we have the original content
storage-cli storage get docs/example.txt
```

### Secure sharing between federations

```bash
# Create federations
storage-cli federation create "Engineering" --description "Engineering team"
storage-cli federation create "Marketing" --description "Marketing team"

# Create an encryption key for both federations
storage-cli encryption create-key engineering-fed marketing-fed

# Store encrypted data accessible by both federations
storage-cli storage put shared/roadmap.txt --file roadmap.txt --encrypted --federation engineering-fed --federation marketing-fed
```

### Collecting and analyzing metrics

```bash
# Set up a basic monitoring script
#!/bin/bash
OUTPUT_DIR="metrics"
mkdir -p $OUTPUT_DIR

# Collect metrics every hour
while true; do
  TIMESTAMP=$(date +%Y%m%d-%H%M%S)
  storage-cli metrics export $OUTPUT_DIR/metrics-$TIMESTAMP.csv --format csv
  
  # Display current stats
  storage-cli metrics show
  
  sleep 3600
done
```

## Error Handling

The CLI provides descriptive error messages to help diagnose issues:

- Permission errors: Occur when your node doesn't have the required access rights
- Connectivity errors: Indicate problems reaching other storage peers
- Key not found: The requested data key doesn't exist
- Version not found: The specified version doesn't exist for the key

## Configuration

The CLI stores its configuration in the data directory specified during initialization, typically in `data/storage/config.json`. 