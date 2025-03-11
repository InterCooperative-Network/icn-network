# Build stage
FROM rust:1.75-slim-bullseye as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/icn
COPY . .

# Build release version
RUN cargo build --release

# Runtime stage
FROM debian:bullseye-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl1.1 \
    procps \
    net-tools \
    openssl \
    && rm -rf /var/lib/apt/lists/*

# Create ICN user and directories
RUN useradd -r -s /bin/false icn && \
    mkdir -p /var/lib/icn /etc/icn/certs /var/log/icn /usr/local/bin/icn && \
    chown -R icn:icn /var/lib/icn /etc/icn /var/log/icn

# Copy the built binary and scripts
COPY --from=builder /usr/src/icn/target/release/icn-node /usr/local/bin/
COPY scripts/generate_certs.sh /usr/local/bin/icn/
COPY scripts/healthcheck.sh /usr/local/bin/icn/
COPY config/node.yaml.template /etc/icn/

# Make scripts executable
RUN chmod +x /usr/local/bin/icn-node \
    /usr/local/bin/icn/generate_certs.sh \
    /usr/local/bin/icn/healthcheck.sh

# Set environment variables
ENV ICN_LOG_LEVEL=info \
    ICN_DATA_DIR=/var/lib/icn \
    ICN_CERT_DIR=/etc/icn/certs \
    ICN_LOG_DIR=/var/log/icn

# Add health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD ["/usr/local/bin/icn/healthcheck.sh"]

# Expose ports
EXPOSE 9000

# Create entrypoint script
RUN echo '#!/bin/bash\n\
/usr/local/bin/icn/generate_certs.sh\n\
exec /usr/local/bin/icn-node "$@"' > /usr/local/bin/icn/entrypoint.sh && \
chmod +x /usr/local/bin/icn/entrypoint.sh

# Switch to ICN user
USER icn

# Set entrypoint
ENTRYPOINT ["/usr/local/bin/icn/entrypoint.sh"] 