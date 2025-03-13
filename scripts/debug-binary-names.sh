#!/bin/bash
set -e

echo "======= Debugging Binary Names ======="

# Check the package name in Cargo.toml
echo "Package name from Cargo.toml:"
grep "^name" Cargo.toml || echo "No name found in Cargo.toml"

# Build the project in debug mode (faster)
echo "Building project..."
cargo build

# List all binaries in the target directory
echo "Binaries found in target/debug:"
find ./target/debug -type f -executable -not -path "*/deps/*" | grep -v '\.d$'

# Try to find the main binary
echo "Possible main binaries:"
for name in icn-node icn_node icn-network icn_network; do
  if [ -f "./target/debug/$name" ]; then
    echo "Found: $name"
  else
    echo "Not found: $name"
  fi
done

echo "======= End of Debug =======" 