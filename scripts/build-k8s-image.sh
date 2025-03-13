#!/bin/bash
set -e

# Default values
IMAGE_REGISTRY=${IMAGE_REGISTRY:-localhost:5000}
IMAGE_NAME=${IMAGE_NAME:-icn-network}
IMAGE_TAG=${IMAGE_TAG:-latest}
PUSH_IMAGE=false
BUILD_ARGS=""

# Parse command line arguments
while [[ $# -gt 0 ]]; do
  case $1 in
    --registry)
      IMAGE_REGISTRY="$2"
      shift 2
      ;;
    --tag)
      IMAGE_TAG="$2"
      shift 2
      ;;
    --push)
      PUSH_IMAGE=true
      shift
      ;;
    --no-push)
      PUSH_IMAGE=false
      shift
      ;;
    --build-arg)
      BUILD_ARGS="$BUILD_ARGS --build-arg $2"
      shift 2
      ;;
    *)
      echo "Unknown option: $1"
      exit 1
      ;;
  esac
done

# Full image name
FULL_IMAGE_NAME="${IMAGE_REGISTRY}/${IMAGE_NAME}:${IMAGE_TAG}"

echo "Building Docker image: ${FULL_IMAGE_NAME}"
echo "This may take several minutes..."

# Try to build the image with better error handling
if ! docker build -t "${FULL_IMAGE_NAME}" -f Dockerfile.k8s ${BUILD_ARGS} .; then
  echo "Docker build failed!"
  echo "Possible solutions:"
  echo "1. Check your Dockerfile.k8s for errors"
  echo "2. Make sure you have enough disk space"
  echo "3. Ensure your Rust/Cargo dependencies are compatible"
  exit 1
fi

echo "Docker build completed successfully!"

if [ "${PUSH_IMAGE}" = "true" ]; then
  echo "Pushing Docker image: ${FULL_IMAGE_NAME}"
  if ! docker push "${FULL_IMAGE_NAME}"; then
    echo "Docker push failed!"
    echo "Possible solutions:"
    echo "1. Make sure you're authenticated to the registry"
    echo "2. Check that the registry is accessible"
    echo "3. Verify you have permission to push to this repository"
    exit 1
  fi
  echo "Docker push completed successfully!"
else
  echo "Skipping push (use --push to push the image)"
fi

echo "Done!" 