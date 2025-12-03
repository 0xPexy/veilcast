FROM rust:trixie AS builder

WORKDIR /app
RUN apt-get update && apt-get install -y --no-install-recommends pkg-config libssl-dev ca-certificates nodejs npm && rm -rf /var/lib/apt/lists/*

# Leverage caching
COPY Cargo.toml Cargo.lock package.json ./
COPY scripts ./scripts
COPY src ./src

ENV RUSTFLAGS="-C target-cpu=native"
RUN cargo build --release
RUN npm install --omit=dev

FROM debian:12-slim AS runtime
WORKDIR /app
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates nodejs npm && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/veilcast-backend /usr/local/bin/veilcast-backend
COPY --from=builder /app/scripts ./scripts
COPY --from=builder /app/package.json ./ 
COPY --from=builder /app/node_modules ./node_modules

ENV RUST_LOG=info
EXPOSE 8000
CMD ["veilcast-backend"]

# Dev image for cargo watch (used by docker compose dev profile)
FROM rust:trixie AS dev
WORKDIR /app
RUN apt-get update && apt-get install -y --no-install-recommends pkg-config libssl-dev ca-certificates nodejs npm && rm -rf /var/lib/apt/lists/*
