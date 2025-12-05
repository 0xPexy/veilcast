# VeilCast

zk-powered anonymous forecasting, XP, and “prophet” ranks — without real‑money betting.

VeilCast is a small playground for making bold predictions **without exposing your identity or wallet**, then earning **XP and ranks** when you’re right.

---

## Highlights (for blockchain / infra engineers)

- **EVM smart contracts with zk integration**  
  - `VeilCastPolls.sol` implements commit–reveal polls, nullifier-based 1p1v, and a Noir-generated Solidity verifier for zk proofs.  
  - Batch reveal path (`batchReveal`) shows how to design gas-aware on-chain aggregation.

- **Off-chain backend, indexer, and relayer in Rust**  
  - Axum + SQLx API server (poll CRUD, commit, membership, profile, leaderboard).  
  - `ethers-rs` WebSocket indexer consumes on-chain events and keeps Postgres in sync.  
  - A background relayer submits batched reveal transactions and records tx hashes.

- **ZK membership and commitments**  
  - Poseidon2-based Merkle tree over `identity_secret`s for anonymous group membership.  
  - Noir circuit proves membership + commitment/nullifier consistency + choice validity (0/1).

- **Frontend that actually generates proofs**  
  - React + TypeScript app uses `@noir-lang/noir_js` and `@aztec/bb.js` in the browser to build proofs before sending them to the backend.  
  - Polymarket-style UX for commit/reveal, plus XP/profile/leaderboard views.

- **Containerized monorepo**  
  - Contracts, backend, frontend, and Postgres wired together via Docker Compose for reproducible local runs and demos.

---

## Demo

- 🎥 YouTube demo:  
  https://www.youtube.com/watch?v=pXMQvaYvws0

The demo walks through a full end‑to‑end flow:

- Creating polls about live BTC/ETH market behavior  
- Client‑side proof generation (Noir + bb.js in the browser)  
- On‑chain reveal via a relayer, and  
- XP scoring + leaderboard updates in the backend.

---

## Product tour (screenshots)

All screenshots live under `images/` in this repo.

### Home — explore active polls

![VeilCast Home](images/veilcast_home.jpg)

- See active and resolved polls grouped by category (macro, crypto, sports, culture, …).
- For each poll you get:
  - Question, category, and phase (Commit / Reveal / Resolved)
  - Remaining time until the next phase
  - A quick status line (e.g. “Reveal in progress, waiting for relayer”).

From here you jump into a specific poll detail page.

---

### Poll detail — question, phases, and eligibility

![Poll Detail](images/veilcast_poll.jpg)

- Full question text and options (binary in v1: e.g. *Yes / No*).
- Clear timeline: commit end, reveal end, resolution status.
- Membership snapshot:
  - The backend computes a Poseidon‑based Merkle root over the currently eligible users.
  - The UI tells you whether your account is in the snapshot and allowed to vote.

If you’re eligible and the poll is in the commit phase, you can lock in a vote.

---

### Commit — lock in an anonymous vote

![Commit](images/veilcast_commit.jpg)

In the commit phase:

- You pick an option (e.g. *Yes* / *No*) via simple buttons, not raw indices.
- The client:
  - Fetches your identity secret and poll‑specific secret from the backend,
  - Computes a zk‑friendly commitment and nullifier using Poseidon2,
  - Generates a Noir proof **in the browser**, and
  - Sends the commitment + proof to the backend.
- The backend stores commitments in Postgres and enforces:
  - One commitment per poll per user (off‑chain 1p1v safety net),
  - Commit window checks against poll deadlines.

A single user can commit once per poll; the UI shows when you’ve already committed.

---

### Reveal — relayer batch submit on-chain

![Reveal](images/veilcast_reveal.jpg)

After the commit window closes:

- A Rust relayer process:
  - Reads unrevealed commitments from the DB,
  - Batches them into a `batchReveal` call to the `VeilCastPolls` contract,
  - Submits a single reveal transaction on‑chain.
- The contract:
  - Verifies each Noir proof with the Solidity verifier,
  - Checks nullifiers to prevent double voting,
  - Increments per‑option vote counts.

The UI links to the reveal transaction on Etherscan (e.g. `View reveal tx`) so you can verify the on‑chain side of the flow.

---

### Resolve — set the outcome and finalize scores

![Resolve](images/veilcast_resolve.jpg)

Once the real‑world outcome is known and the reveal phase is done:

- Only the poll owner can call `resolve`:
  - Picks the correct option index,
  - The contract marks the poll as resolved.
- The backend:
  - Reads final vote counts per option,
  - Applies a simple XP policy:
    - Participation XP for everyone who voted,
    - Extra XP for those who picked the correct option.
  - Updates `user_stats` (XP, total votes, correct votes, tier).

The poll detail page then shows:

- Final percentages (e.g. `Yes 67% / No 33%`), and  
- A clear banner stating the correct outcome.

---

### Profile — your anonymous forecasting track record

![Profile](images/veilcast_profile.jpg)

The profile page is your anonymous “forecasting CV”:

- XP total and current tier (e.g. *Apprentice*, *Gold Seer*, *Master Oracle*).
- Total votes vs correct votes and derived win‑rate.
- A list of recent polls you participated in, with:
  - Your choice,
  - Final outcome,
  - XP gained.

Wallets/identities stay behind ZK primitives; the profile only exposes performance.

---

### Leaderboard — top anonymous forecasters

![Leaderboard](images/veilcast_leaderboard.jpg)

The leaderboard page aggregates `user_stats`:

- Sorted by XP (tie‑breakers by win‑rate).
- Shows:
  - Username (pseudonymous),
  - XP,
  - Tier,
  - Total and correct votes.

You can see the overall “prophet food chain” without ever revealing who the humans are behind each handle.

---

## How VeilCast works (conceptually)

1. **Membership & identity**
   - The backend maintains a set of members, each with an `identity_secret`.
   - When a poll is created, the backend takes a snapshot:
     - Poseidon2‑based Merkle tree over current `identity_secret`s,
     - Stores the root as `membershipRoot` on‑chain.
   - The Noir circuit proves:
     - You are in that Merkle tree,
     - Without revealing your exact leaf.

2. **Commit**
   - Client chooses `choice ∈ {0,1}`.
   - Computes:
     - `commitment = Poseidon2(choice, secret)`
     - `nullifier  = Poseidon2(identity_secret, poll_id)`
   - Sends `(commitment, nullifier, proof)` to the backend; backend stores commitment off‑chain.

3. **Reveal (relayer)**
   - After commit ends, the relayer:
     - Fetches unrevealed commitments,
     - Calls the on‑chain `batchReveal` with proof bytes and public inputs.
   - Contract:
     - Verifies proofs,
     - Checks `nullifierUsed[pollId][nullifier] == false`,
     - Increments `votes[pollId][choiceIndex]`.

4. **Resolve & XP**
   - Poll owner sets `correctOption`.
   - Backend walks all commitments for that poll:
     - Updates XP and tiers in `user_stats`.

---

## Tech stack

- **Contracts**
  - `VeilCastPolls.sol` (Foundry / Solidity, commit–reveal polls, 1p1v via nullifiers)
  - Noir‑generated Solidity verifier (`Verifier.sol`) for the zk circuit

- **ZK**
  - Noir circuit for:
    - Merkle membership
    - Commitment/nullifier consistency
    - Choice validity (0/1)
  - `@noir-lang/noir_js` + `@aztec/bb.js` in the browser to generate proofs

- **Backend**
  - Rust (`axum`, `sqlx`, `ethers-rs`)
  - Responsibilities:
    - API server (poll CRUD, commit, membership, profile, leaderboard)
    - ZK helper integration and membership Merkle path generation
    - On‑chain indexer + reveal relayer
    - XP engine + leaderboard and backfill jobs

- **Frontend**
  - React + TypeScript + Vite
  - Tailwind CSS for layout / styling
  - Wagmi/web3 stack prepared for wallet integrations (v1 is username/password demo login)

- **Infra**
  - Docker + docker‑compose (`infra/docker-compose.yml`)
  - Separate containers for:
    - Postgres
    - Rust backend
    - React frontend
  - Local Noir/bb tooling under `zk/` and `contracts/script/zk`.

---

## Dev quickstart

From the monorepo root:

```bash
# Start everything via docker-compose
cd infra
docker compose up --build
```

Or run each piece manually:

```bash
# Postgres via docker, for example
cd infra
docker compose up -d db

# Backend
cd backend
DATABASE_URL=postgres://veilcast:veilcast@localhost:5432/veilcast cargo run

# Frontend
cd frontend
npm install
npm run dev
```

Notes:

- Backend config is driven by `infra/.env.backend` (RPC URLs, contract addresses, etc.).
- Frontend config is in `infra/.env.frontend` (API base URL).
- For contract work:
  - Build a Foundry image: `docker build -t veilcast-foundry contracts`
  - Run tests: `docker run --rm -it -v $(pwd)/contracts:/app veilcast-foundry forge test`

For more backend‑specific details (indexer, XP engine, demo seeding), see `backend/README.md`. For zk circuit and verifier generation, see `zk/` and `contracts/script/zk`.
