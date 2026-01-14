# Build stage
FROM rust:1.92 AS builder

WORKDIR /app

# Copy workspace Cargo files
COPY Cargo.toml ./
COPY core ./core
COPY contrib ./contrib
COPY examples/observability_telemetry ./examples/observability_telemetry

# Build the application
WORKDIR /app/examples/observability_telemetry
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install CA certificates and curl for healthchecks
RUN apt-get update && \
    apt-get install -y ca-certificates curl && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy only the compiled binary from builder
COPY --from=builder /app/examples/observability_telemetry/target/release/observability-telemetry-example .

# Expose port
EXPOSE 3000

# Run the server
CMD ["./observability-telemetry-example"]
