# ICN Development Environment Setup

This guide walks through setting up a complete development environment for the Intercooperative Network.

## Prerequisites

- **Rust** - ICN is primarily implemented in Rust
- **Docker** - For containerized development and testing
- **Git** - For version control
- **IPFS** - For distributed storage testing
- **Physical Test Devices** (optional) - For mesh network testing

## Step 1: Install Rust and Cargo

```bash
# Install rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add the nightly channel for some features
rustup install nightly
rustup default stable

# Install useful Rust components
rustup component add clippy rustfmt

# Install cargo-make for build automation
cargo install cargo-make

# Install cargo-audit for security checking
cargo install cargo-audit
```

## Step 2: Set Up Docker Environment

```bash
# Install Docker (Ubuntu example)
sudo apt update
sudo apt install -y docker.io docker-compose

# Add your user to the docker group
sudo usermod -aG docker $USER

# Log out and back in for the group change to take effect
```

## Step 3: Clone and Initialize the Repository

```bash
# Clone the repository
git clone https://github.com/icn/intercooperative-network.git
cd intercooperative-network

# Initialize git submodules
git submodule update --init --recursive

# Install project-specific dependencies
cargo make setup
```

## Step 4: Install IPFS for Storage Testing

```bash
# Install IPFS (Ubuntu example)
wget https://dist.ipfs.io/go-ipfs/v0.12.0/go-ipfs_v0.12.0_linux-amd64.tar.gz
tar -xvzf go-ipfs_v0.12.0_linux-amd64.tar.gz
cd go-ipfs
sudo bash install.sh
ipfs init
```

## Step 5: Set Up the Development Container

```bash
# Start the development container
docker-compose up -d dev

# Shell into the container
docker-compose exec dev bash
```

## Step 6: Configure the Test Network

```bash
# Initialize a local test network
cargo make test-network-init

# Start the test network
cargo make test-network-start
```

## Step 7: Install IDE Extensions

### Visual Studio Code
Install the following extensions:
- rust-analyzer
- Better TOML
- crates
- CodeLLDB
- Docker
- Remote - Containers

### IntelliJ/CLion
Install the following plugins:
- Rust
- TOML
- Docker
- Protobuf

## Step 8: Set Up Hardware for Mesh Testing (Optional)

For testing mesh networking capabilities:

1. Set up Raspberry Pi devices or similar hardware
2. Install the ICN mesh network components:
   ```bash
   # Install on Raspberry Pi
   curl -sSL https://get.icn.coop/mesh | bash
   ```

3. Configure the mesh network:
   ```bash
   # Configure mesh networking
   icn-mesh config --mode=adhoc --interface=wlan0
   ```

## Development Workflow

### Build the Project

```bash
# Build all components
cargo build

# Build specific component
cargo build -p icn-identity-system
```

### Run Tests

```bash
# Run all tests
cargo test

# Run specific component tests
cargo test -p icn-governance-system
```

### Run Simulations

```bash
# Run a network simulation
cargo make simulate-network

# Run a federation simulation
cargo make simulate-federation
```

### Run Specific Components

```bash
# Run a local node
cargo run --bin icn-node -- --config=local.toml

# Run with specific features
cargo run --bin icn-node --features="mesh-network" -- --config=mesh.toml
```

### Using the Development Container

The development container provides a consistent environment with all dependencies pre-installed.

```bash
# Build inside the container
docker-compose exec dev cargo build

# Run tests inside the container
docker-compose exec dev cargo test

# Add a new dependency
docker-compose exec dev cargo add some-package
```

### Debugging

For debugging, you can use:

1. **VS Code with CodeLLDB**:
   - Set breakpoints in the editor
   - Use the launch configuration in `.vscode/launch.json`

2. **Command-line debugging**:
   ```bash
   # Set RUST_BACKTRACE for detailed errors
   RUST_BACKTRACE=1 cargo run --bin icn-node
   ```

3. **Logging**:
   - Set log levels with `RUST_LOG=debug`
   - Use the structured logging macros in the codebase

### Profiling and Benchmarking

```bash
# Run benchmarks
cargo bench

# Profile a component
cargo flamegraph --bin icn-node
```

## Multi-platform Development

ICN is designed to run on various platforms. To build for different targets:

```bash
# Add a target
rustup target add aarch64-unknown-linux-gnu

# Build for Raspberry Pi
cargo build --target aarch64-unknown-linux-gnu

# Build for Android (requires NDK)
cargo build --target aarch64-linux-android
```

## Troubleshooting

### Common Issues

1. **Compiler errors due to missing system dependencies**:
   ```bash
   # Install common system dependencies (Ubuntu example)
   sudo apt install -y build-essential pkg-config libssl-dev libsqlite3-dev
   ```

2. **Network errors in tests**:
   - Check that the test network is running: `cargo make test-network-status`
   - Reset the test network if needed: `cargo make test-network-reset`

3. **Mesh network issues**:
   - Ensure wireless interfaces are in the correct mode: `sudo iwconfig`
   - Check for interference on wireless channels
   - Verify hardware supports ad-hoc mode

### Getting Help

- Join the ICN developer chat: [https://chat.icn.coop](https://chat.icn.coop)
- Submit issues to the repository: [https://github.com/icn/intercooperative-network/issues](https://github.com/icn/intercooperative-network/issues)
- Consult the developer documentation: [https://docs.icn.coop/developer](https://docs.icn.coop/developer)
