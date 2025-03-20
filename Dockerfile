# Multi-architecture Dockerfile for ICN node
# Support for multiple build types:
# - BUILDTYPE=default (regular build with actual ICN node)
# - BUILDTYPE=simple (simple placeholder for testing)
# - BUILDTYPE=k8s (optimized for Kubernetes)

ARG BUILDTYPE=default
ARG RUST_VERSION=1.70
ARG DEBIAN_VERSION=bookworm-slim

# Build stage
FROM rust:${RUST_VERSION}-slim as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/icn
COPY . .

# Build release version with optimizations
RUN cargo build --release

# Runtime stage
FROM debian:${DEBIAN_VERSION} as base

# Install common runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    procps \
    net-tools \
    openssl \
    gettext-base \
    && rm -rf /var/lib/apt/lists/*

# Create ICN user and directories
RUN useradd -r -s /bin/false icn && \
    mkdir -p /var/lib/icn /etc/icn/certs /var/log/icn /usr/local/bin/icn && \
    chown -R icn:icn /var/lib/icn /etc/icn /var/log/icn

# Set up common environment variables
ENV ICN_LOG_LEVEL=info \
    ICN_DATA_DIR=/var/lib/icn \
    ICN_CERT_DIR=/etc/icn/certs \
    ICN_LOG_DIR=/var/log/icn

# Expose ports
EXPOSE 9000 9001 9002

# Default configuration
FROM base as default
COPY --from=builder /usr/src/icn/target/release/icn-node /usr/local/bin/icn-node
RUN chmod +x /usr/local/bin/icn-node

# Create entrypoint script
RUN echo '#!/bin/bash\n\
echo "Starting ICN node..."\n\
exec /usr/local/bin/icn-node "$@"\n\
' > /usr/local/bin/icn/entrypoint.sh && \
chmod +x /usr/local/bin/icn/entrypoint.sh

# Add health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:9000/health || exit 1

# Simple configuration (for testing)
FROM base as simple

# Create a simple healthcheck script
RUN echo '#!/bin/bash\n\
echo "ICN node is healthy"\n\
exit 0\n\
' > /usr/local/bin/icn/healthcheck.sh && \
chmod +x /usr/local/bin/icn/healthcheck.sh

# Create a simple entrypoint script
RUN echo '#!/bin/bash\n\
echo "ICN node starting..."\n\
echo "Node ID: ${ICN_NODE_ID:-node-0}"\n\
echo "Coop ID: ${ICN_COOP_ID:-coop-0}"\n\
echo "Node Type: ${ICN_NODE_TYPE:-primary}"\n\
echo "Listen Address: ${ICN_LISTEN_ADDR:-0.0.0.0:9000}"\n\
\n\
# Keep the container running\n\
while true; do\n\
  echo "ICN node is running..."\n\
  sleep 60\n\
done\n\
' > /usr/local/bin/icn/entrypoint.sh && \
chmod +x /usr/local/bin/icn/entrypoint.sh

# Add health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD ["/usr/local/bin/icn/healthcheck.sh"]

# Kubernetes optimized configuration
FROM base as k8s

# Copy the main binary
COPY --from=builder /usr/src/icn/target/release/icn-node /usr/local/bin/icn-node
RUN chmod +x /usr/local/bin/icn-node

# Copy scripts if they exist, or create them
COPY scripts/generate_certs.sh /usr/local/bin/icn/ || true
COPY scripts/healthcheck.sh /usr/local/bin/icn/ || true

# Create default script versions if not copied
RUN if [ ! -f /usr/local/bin/icn/generate_certs.sh ]; then \
    echo '#!/bin/bash\n\
if [ ! -f "${ICN_CERT_DIR}/node.crt" ]; then\n\
  echo "Generating self-signed certificates..."\n\
  openssl req -x509 -newkey rsa:4096 -keyout "${ICN_CERT_DIR}/node.key" -out "${ICN_CERT_DIR}/node.crt" -days 365 -nodes -subj "/CN=${ICN_NODE_ID:-node-0}/O=${ICN_COOP_ID:-coop-0}"\n\
  cp "${ICN_CERT_DIR}/node.crt" "${ICN_CERT_DIR}/ca.crt"\n\
  echo "Certificates generated."\n\
else\n\
  echo "Certificates already exist. Skipping generation."\n\
fi\n\
' > /usr/local/bin/icn/generate_certs.sh; \
    chmod +x /usr/local/bin/icn/generate_certs.sh; \
    fi

RUN if [ ! -f /usr/local/bin/icn/healthcheck.sh ]; then \
    echo '#!/bin/bash\n\
if [ -f /usr/local/bin/icn-node ]; then\n\
  # Try to get health info from the node\n\
  curl -sf http://localhost:9000/health && exit 0 || echo "Health check failed, checking if process is running..."\n\
  # Check if process is running\n\
  pgrep -f "icn-node" && exit 0 || exit 1\n\
else\n\
  echo "ICN node binary not found"\n\
  exit 1\n\
fi\n\
' > /usr/local/bin/icn/healthcheck.sh; \
    chmod +x /usr/local/bin/icn/healthcheck.sh; \
    fi

# Create entrypoint script with improved environment variable handling
RUN echo '#!/bin/bash\n\
# Generate certificates if they do not exist\n\
/usr/local/bin/icn/generate_certs.sh\n\
\n\
# Process template with environment variables\n\
if [ -f /etc/icn/node.yaml.template ]; then\n\
  # Export variables with defaults for envsubst\n\
  export ICN_NODE_ID=${ICN_NODE_ID:-node-0}\n\
  export ICN_COOP_ID=${ICN_COOP_ID:-coop-0}\n\
  export ICN_NODE_TYPE=${ICN_NODE_TYPE:-primary}\n\
  export ICN_LISTEN_ADDR=${ICN_LISTEN_ADDR:-0.0.0.0:9000}\n\
  export ICN_PEERS=${ICN_PEERS:-[]}\n\
  export ICN_DATA_DIR=${ICN_DATA_DIR:-/var/lib/icn}\n\
  export ICN_CERT_DIR=${ICN_CERT_DIR:-/etc/icn/certs}\n\
  export ICN_LOG_DIR=${ICN_LOG_DIR:-/var/log/icn}\n\
  export ICN_LOG_LEVEL=${ICN_LOG_LEVEL:-info}\n\
  \n\
  # Process the template\n\
  envsubst < /etc/icn/node.yaml.template > /etc/icn/node.yaml\n\
  echo "Configuration file created at /etc/icn/node.yaml"\n\
else\n\
  echo "Warning: Configuration template not found at /etc/icn/node.yaml.template"\n\
fi\n\
\n\
# Start the node\n\
echo "Starting ICN node..."\n\
exec /usr/local/bin/icn-node "$@"\n\
' > /usr/local/bin/icn/entrypoint.sh && \
chmod +x /usr/local/bin/icn/entrypoint.sh

# Add health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD ["/usr/local/bin/icn/healthcheck.sh"]

# Select the appropriate final image based on BUILDTYPE
FROM ${BUILDTYPE} as final

# Switch to ICN user
USER icn

# Set entrypoint
ENTRYPOINT ["/usr/local/bin/icn/entrypoint.sh"] 