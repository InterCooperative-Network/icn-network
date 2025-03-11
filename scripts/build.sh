#!/bin/bash
set -e

# Display banner
echo "====================================="
echo "ICN Network - Build Script"
echo "====================================="

# Check if cargo is installed
if ! command -v cargo &> /dev/null; then
    echo "Cargo is not installed. Please install Rust and Cargo first."
    exit 1
fi

# Build with different configurations
echo "Building debug version..."
cargo build

echo "Running tests..."
cargo test

echo "Building optimized release version..."
cargo build --release

echo "Build completed successfully!"
echo "=====================================" 