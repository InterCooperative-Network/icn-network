# Build stage
FROM rust:1.70-slim as builder

# Create a new empty shell project
WORKDIR /usr/src/icn-node
RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev && \
    rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src

# Build the application
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y ca-certificates libssl3 && \
    rm -rf /var/lib/apt/lists/*

# Set up directories
RUN mkdir -p /etc/icn/certs /var/lib/icn /var/log/icn

# Copy the built executable
COPY --from=builder /usr/src/icn-node/target/release/icn-node /usr/local/bin/icn-node

# Set environment variables
ENV ICN_LOG_LEVEL=info \
    ICN_DATA_DIR=/var/lib/icn \
    ICN_CERT_DIR=/etc/icn/certs \
    ICN_LOG_DIR=/var/log/icn

# Create a non-root user to run the application
RUN groupadd -r icn && useradd -r -g icn icn && \
    chown -R icn:icn /etc/icn /var/lib/icn /var/log/icn

# Switch to non-root user
USER icn

# Define the command to run
ENTRYPOINT ["icn-node"]

# Expose the P2P port
EXPOSE 9000

# Add health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:9000/health || exit 1 