FROM rust:trixie AS builder

WORKDIR /app
RUN apt-get update && apt-get install -y --no-install-recommends pkg-config libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*

# Leverage caching
COPY Cargo.toml Cargo.lock ./
COPY src ./src

ENV RUSTFLAGS="-C target-cpu=native"
RUN cargo build --release

FROM debian:12-slim
WORKDIR /app
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/veilcast-backend /usr/local/bin/veilcast-backend

ENV RUST_LOG=info
EXPOSE 8000
CMD ["veilcast-backend"]
