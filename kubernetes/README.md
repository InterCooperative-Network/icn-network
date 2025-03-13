# Kubernetes Deployment for ICN Network

This directory contains Kubernetes configurations to deploy the Intercooperative Network (ICN) to a Kubernetes cluster.

## Prerequisites

- A running Kubernetes cluster (local like Minikube, KinD, or a cloud-based cluster)
- kubectl configured to communicate with your cluster
- Docker installed on your local machine for building the image

## Directory Structure

```
kubernetes/
├── namespace.yaml                    # Kubernetes namespace definition
├── configmap.yaml                    # Configuration for ICN nodes
├── persistent-volume-claims.yaml     # PVCs for node data, certs, and logs
├── coop1-primary-deployment.yaml     # Deployment for coop1-primary node
├── coop1-primary-service.yaml        # Service for coop1-primary node
├── coop1-secondary-deployment.yaml   # Deployment for coop1-secondary node
├── coop1-secondary-service.yaml      # Service for coop1-secondary node
├── coop2-primary-deployment.yaml     # Deployment for coop2-primary node
├── coop2-primary-service.yaml        # Service for coop2-primary node
├── coop2-secondary-deployment.yaml   # Deployment for coop2-secondary node
├── coop2-secondary-service.yaml      # Service for coop2-secondary node
```

## Building the Docker Image for Kubernetes

A specialized Dockerfile (`Dockerfile.k8s`) has been created for Kubernetes deployments that includes optimizations and configurations specific to container orchestration.

### Using a Remote Container Registry

To build and push the image to a remote registry:

```bash
# Set your container registry
export IMAGE_REGISTRY=your-registry.io
export IMAGE_TAG=v0.1.0

# Build and push the image
./scripts/build-k8s-image.sh --registry ${IMAGE_REGISTRY} --tag ${IMAGE_TAG} --push
```

### Using a Local Image

For local development or testing, you can build and use a local image without pushing to a registry:

```bash
# Build a local image
docker build -t icn-network:latest -f Dockerfile.k8s .
```

## Deploying to Kubernetes

Use the provided deployment script to deploy the ICN network to Kubernetes:

### Remote Registry Deployment

```bash
# Build and deploy in one step
./scripts/deploy-to-k8s.sh --registry ${IMAGE_REGISTRY} --tag ${IMAGE_TAG} --build

# Or just deploy if you've already built the image
./scripts/deploy-to-k8s.sh --registry ${IMAGE_REGISTRY} --tag ${IMAGE_TAG}
```

### Local Development Deployment

For local development using Minikube or KinD, you can use:

```bash
# Deploy using a local image
./scripts/deploy-to-k8s.sh --local
```

This will:
1. Build the image locally
2. Configure the deployments to use the local image
3. Set the imagePullPolicy to IfNotPresent (so Kubernetes doesn't try to pull from a registry)

### Minikube Deployment

For Minikube, we've added special support to make deployment easier:

```bash
# Deploy to Minikube
./scripts/deploy-to-k8s.sh --minikube
```

This will:
1. Configure Docker to use Minikube's Docker daemon (`eval $(minikube docker-env)`)
2. Build the image directly in Minikube's Docker environment
3. Set imagePullPolicy to Never
4. Configure service access via Minikube

### Additional Options

The deployment script supports several additional options:

```bash
# Clean up existing deployment before deploying
./scripts/deploy-to-k8s.sh --local --clean

# Specify a Kubernetes context
./scripts/deploy-to-k8s.sh --local --context my-context

# Specify a different namespace
./scripts/deploy-to-k8s.sh --local --namespace icn-test
```

## Troubleshooting Deployment Issues

### Image Pull Errors

If you encounter image pull errors, check the following:

1. For remote registries: Ensure the registry is accessible and you have proper authentication
2. For local deployments: Ensure you've built the image locally and used the `--local` flag
3. For Minikube: Use the `--minikube` flag to ensure the image is built in Minikube's Docker environment

### Cargo/Rust Version Issues

If you see errors related to Cargo.lock version incompatibilities:

1. We now use `rust:latest` in the Dockerfile to ensure compatibility with modern Cargo.lock formats
2. If building directly, ensure your Rust toolchain is up to date

### Missing Binary Issues

If the Dockerfile can't find the executable:

1. The project now includes a `main.rs` file that produces an executable named `icn-network`
2. The Dockerfile copies this to `/usr/local/bin/icn-node` in the container image
3. If you make changes to the project structure, ensure the binary path in Dockerfile.k8s is updated

## Deployment Strategy

The deployment follows this order:

1. Create namespace and ConfigMap
2. Create persistent volume claims
3. Deploy primary nodes (coop1-primary, coop2-primary)
4. Wait for primary nodes to be ready
5. Deploy secondary nodes (coop1-secondary, coop2-secondary)

This ensures that primary nodes are available before secondary nodes try to connect to them.

## Monitoring the Deployment

To check the status of your deployment:

```bash
kubectl get pods -n icn-network
kubectl get services -n icn-network
```

To view logs from a specific node:

```bash
kubectl logs -n icn-network deployment/coop1-primary
```

## Accessing the Network

The ICN nodes can be accessed within the cluster using their service names:

- coop1-primary.icn-network.svc.cluster.local:9000
- coop1-secondary.icn-network.svc.cluster.local:9001
- coop2-primary.icn-network.svc.cluster.local:9002
- coop2-secondary.icn-network.svc.cluster.local:9003

For Minikube, you can access services using:

```bash
minikube service coop1-primary -n icn-network
```

For production deployments, you can expose the network outside the cluster by modifying the service type to LoadBalancer or NodePort, or creating an Ingress resource.

## Cleanup

To remove the ICN network from your Kubernetes cluster:

```bash
kubectl delete namespace icn-network
```

This will delete all resources created as part of the deployment. 