# Docker Build Guide

This document explains how to use the consolidated Dockerfile to build different types of ICN node images.

## Overview

The consolidated Dockerfile supports three build types:

1. **default** - Standard build with the actual ICN node implementation
2. **simple** - Simple placeholder image for testing connectivity and infrastructure
3. **k8s** - Optimized for Kubernetes deployment with additional configuration options

## Building Docker Images

### Standard Build

```bash
# Build the standard image
docker build -t icn-node:latest .

# Or explicitly specify default build type
docker build --build-arg BUILDTYPE=default -t icn-node:latest .
```

### Simple Build (for testing)

```bash
# Build a simple placeholder image
docker build --build-arg BUILDTYPE=simple -t icn-node:simple .
```

### Kubernetes-optimized Build

```bash
# Build a Kubernetes-optimized image
docker build --build-arg BUILDTYPE=k8s -t icn-node:k8s .
```

## Additional Build Arguments

You can customize builds with these additional arguments:

```bash
# Specify Rust version
docker build --build-arg RUST_VERSION=1.75 -t icn-node:latest .

# Specify Debian version
docker build --build-arg DEBIAN_VERSION=bullseye-slim -t icn-node:latest .

# Combine multiple arguments
docker build \
  --build-arg BUILDTYPE=k8s \
  --build-arg RUST_VERSION=1.75 \
  --build-arg DEBIAN_VERSION=bullseye-slim \
  -t icn-node:custom .
```

## Running the Container

### Basic Usage

```bash
docker run -d --name icn-node -p 9000:9000 icn-node:latest
```

### With Environment Variables

```bash
docker run -d --name icn-node \
  -p 9000:9000 \
  -e ICN_NODE_ID=node-1 \
  -e ICN_COOP_ID=coop-1 \
  -e ICN_NODE_TYPE=primary \
  -e ICN_LISTEN_ADDR=0.0.0.0:9000 \
  icn-node:latest
```

### With Volume Mounts

```bash
docker run -d --name icn-node \
  -p 9000:9000 \
  -v ./config:/etc/icn \
  -v ./data:/var/lib/icn \
  -v ./logs:/var/log/icn \
  icn-node:latest
```

## Environment Variables

The following environment variables can be configured:

| Variable | Description | Default |
|----------|-------------|---------|
| ICN_NODE_ID | Node identifier | node-0 |
| ICN_COOP_ID | Cooperative identifier | coop-0 |
| ICN_NODE_TYPE | Node type (primary/secondary) | primary |
| ICN_LISTEN_ADDR | Listen address for P2P communication | 0.0.0.0:9000 |
| ICN_DATA_DIR | Data directory path | /var/lib/icn |
| ICN_CERT_DIR | Certificate directory path | /etc/icn/certs |
| ICN_LOG_DIR | Log directory path | /var/log/icn |
| ICN_LOG_LEVEL | Log level (debug/info/warn/error) | info |

## Health Checks

All image variants include health checks:

- **default**: Uses HTTP health endpoint at port 9000
- **simple**: Always returns healthy status
- **k8s**: Checks HTTP endpoint and process status as fallback

## Notes

- Previous separate Dockerfiles (`Dockerfile.simple` and `Dockerfile.k8s`) have been consolidated into this single file
- The old Dockerfiles are archived in `archives/docker/` for reference
- This approach reduces maintenance overhead and ensures consistent build processes 