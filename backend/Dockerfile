FROM rust:1.81-slim

WORKDIR /app

# Pre-copy cargo metadata for better layer caching
COPY Cargo.toml Cargo.lock ./

# Copy source
COPY src ./src

# Build in dev mode
RUN cargo build

# Default command
CMD ["cargo", "run"]
