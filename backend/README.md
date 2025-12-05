# VeilCast Backend (Rust)

Lightweight axum-based API and indexing service for VeilCast.

## Responsibilities
- Store poll metadata in Postgres
- Record commitments / nullifiers
- Provide a pluggable ZK backend (`ZkBackend`), currently using a `NoopZkBackend` with SHA‑256 based mock proofs
- Expose HTTP routes: `/health`, `/polls`, `/polls/:id`, `/polls/:id/commit`, `/polls/:id/prove`, `/polls/:id/reveal`

## Running locally
```bash
cd backend
DATABASE_URL=postgres://veilcast:veilcast@localhost:5432/veilcast cargo run
# To also run the on‑chain indexer (requires WS endpoint / contract address):
# RPC_WS=ws://localhost:8545 CONTRACT_ADDRESS=0x... INDEXER_FROM_BLOCK=0 cargo run
```

Or via Docker (from the monorepo root):
```bash
cd infra
docker compose up --build
```

### Demo data seeding
To quickly get a rich demo view (resolved polls + leaderboard), run the seed script from the backend root.  
You need a `psql` client; if `DATABASE_URL` is not set, it defaults to `postgres://veilcast:veilcast@localhost:5432/veilcast`.  
If `psql` is not available locally but Docker is installed, the script will automatically fall back to `docker compose exec db psql`.
```bash
cd backend
chmod +x scripts/seed_demo_data.sh # first time only
DATABASE_URL=postgres://veilcast:veilcast@localhost:5432/veilcast scripts/seed_demo_data.sh
```
The script seeds seven resolved polls around real post‑Sept‑2025 global events (Nvidia #1 by market cap, Apple at $4T, Bitcoin ATH and drawdown, Solana ETF launch, BoA opening up to crypto, 2025 World Series, etc.), along with demo commitments, votes, and leaderboard entries.  
After seeding, you can run `cargo run` or the docker dev profile as usual.

## Tests
```bash
cd backend
cargo test
```

## Architecture notes
- `AppState<Store, Backend>` wires together a `PollStore` implementation (Postgres / in‑memory) and a `ZkBackend` implementation (currently `NoopZkBackend`).
- `PollStore` is a trait abstraction over the DB; `PgStore` manages schema initialization and queries.
- `ZkBackend` encapsulates proof generation / verification, so a real Noir/bb.js backend can replace the mock backend later.
- Poll phase logic uses the current time to validate commit / reveal windows.
- Indexer: an `ethers-rs` WebSocket subscriber pushes on‑chain events into the DB via the `PollIndexSink` trait (`PollCreated`, `VoteRevealed`, `PollResolved`).
