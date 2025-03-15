# Intercooperative Network (ICN)

The Intercooperative Network (ICN) is a decentralized platform designed to enable cooperation and economic interactions between cooperatives and solidarity economy organizations.

## Project Overview

The ICN project aims to create a decentralized infrastructure that supports:

1. **Decentralized Identity (DID)**: Allowing cooperatives to establish and manage their digital identities
2. **Mutual Credit Systems**: Enabling economic exchanges between cooperatives without relying on traditional currencies
3. **Resource Sharing**: Facilitating the sharing of resources between cooperatives
4. **Governance**: Supporting democratic decision-making processes
5. **Reputation System**: Building trust between cooperatives through attestations and behavior tracking

## Repository Structure

The project is organized into modular components:

```
crates/
├── core/
│   ├── icn-common/       # Common utilities and types
│   ├── icn-crypto/       # Cryptographic primitives
├── identity/
│   ├── icn-did/          # Decentralized Identity implementation
│   ├── icn-credentials/  # Verifiable Credentials
├── economic/
│   ├── icn-mutual-credit/ # Mutual Credit implementation
├── storage/
│   ├── icn-storage-system/ # Storage system
examples/                  # Integration examples
standalone/               # Standalone example implementation
```

## Current Status

The project is in early development, with the following components implemented:

- **Core Utilities**: Basic cryptographic primitives and common types
- **Identity System**: DID implementation with local resolution
- **Mutual Credit**: Basic mutual credit system with account management and transactions
- **Integration Example**: Demonstrating the interaction between identity and mutual credit systems
- **Reputation System**: Trust scoring based on economic transactions and governance participation
- **Governance System**: Federation governance with proposals, voting, and deliberation processes

## Key Features

### Reputation System

The reputation system provides a framework for building trust within the network by:

- **Attestations**: Recording positive or negative actions by network participants
- **Trust Scores**: Computing trust scores based on historical interactions
- **Sybil Resistance**: Detecting potential Sybil attacks by analyzing account patterns
- **Integration with Economic Systems**: Adjusting credit limits based on reputation
- **Governance Participation**: Rewarding active participation in governance

### Governance & Deliberation

The governance system enables democratic decision-making within federations:

- **Proposal Creation**: Members can create governance proposals
- **Voting**: Democratic voting on proposals
- **Deliberation**: Structured discussions around proposals
- **Quality Assessment**: Evaluation of deliberation quality based on depth and references
- **Reputation Integration**: Building reputation through governance participation

## Getting Started

### Running the Standalone Example

For a quick demonstration of the core concepts:

```bash
cd standalone
cargo run
```

This example demonstrates the integration between the identity system and mutual credit systems in a simplified manner.

### Running the Integration Example

To run the full integration example (requires all dependencies):

```bash
cargo run --example identity_and_credit
```

## Development Roadmap

The project is following a phased development approach:

1. **Phase 1 (Current)**: Core infrastructure and basic implementations
   - Identity system with DIDs
   - Basic mutual credit implementation
   - Integration between components
   - Reputation and governance systems

2. **Phase 2 (Planned)**: Enhanced functionality
   - Federation of DIDs across networks
   - Advanced credit policies and governance
   - Resource sharing mechanisms
   - Advanced reputation metrics and analysis

3. **Phase 3 (Future)**: Network deployment
   - Decentralized network deployment
   - Integration with existing cooperative networks
   - User-friendly interfaces
   - Reputation visualization and recommendation systems

## Contributing

The project is open for contributions. Key areas where help is needed:

- Implementing missing components
- Enhancing existing implementations
- Documentation and examples
- Testing and security reviews

## License

This project is licensed under the MIT OR Apache-2.0 license.

# ICN Network Deployment

This repository contains scripts and configuration files for deploying the ICN (Inter-Cooperative Network) on a Kubernetes cluster.

## Prerequisites

- A Kubernetes cluster with at least one master node
- Docker installed on the local machine
- SSH access to the Kubernetes master node

## Directory Structure

- `kubernetes/`: Contains Kubernetes YAML files for deployment
- `scripts/`: Contains shell scripts for deployment and management
- `config/`: Contains configuration templates for the ICN nodes
- `Dockerfile.simple`: Dockerfile for building the ICN node image

## Deployment Process

1. **Configure Registry**

   ```bash
   ./scripts/configure-registry.sh
   ```

   This script configures the HTTP registry on the Kubernetes cluster.

2. **Build and Push Docker Image**

   ```bash
   ./scripts/push-to-all-nodes.sh
   ```

   This script builds the Docker image, saves it to a tar file, transfers it to the remote server, loads it on the master node, tags it for the local registry, and pushes it to the registry.

3. **Deploy ICN Network**

   ```bash
   ./scripts/deploy-master-only.sh
   ```

   This script deploys the ICN network on the master node only.

4. **Check Deployment Status**

   ```bash
   ssh -t -i /home/matt/.ssh/id_rsa_new matt@10.10.100.102 "sudo kubectl get all -n icn-system"
   ```

   This command checks the status of all resources in the icn-system namespace.

## Cleanup

To clean up old deployments:

```bash
./scripts/cleanup-old-deployments.sh
```

This script deletes old deployments and services in the icn-system namespace.

## Troubleshooting

If you encounter issues with the deployment, check the logs of the pods:

```bash
ssh -t -i /home/matt/.ssh/id_rsa_new matt@10.10.100.102 "sudo kubectl logs -n icn-system <pod-name>"
```

Replace `<pod-name>` with the name of the pod you want to check.

## Notes

- The ICN network is currently deployed only on the master node due to issues with image pulling on worker nodes.
- The deployment uses a simplified Docker image that simulates the ICN node behavior.
- The configuration is mounted as a ConfigMap in the deployment.
