# VeilCast

Anonymous forecasting, XP, and “prophet” ranks — without real-money betting.

VeilCast is a lightweight prediction & voting playground where people can make bold forecasts **without exposing their identity or wallet**, then earn **XP and ranks** when they’re right.

---

## What is VeilCast?

VeilCast lets you:

- Join **prediction polls** on real-world or crypto events  
  (e.g. *“BTC will hit 150k before 2027”*, *“Layer X will flip Layer Y in TVL”*).
- Vote **anonymously**: your choice is hidden during the commit period.
- See results only **after the poll closes** (commit–reveal).
- Gain **XP** when your prediction is correct.
- Climb through **fun ranks** (e.g. *“Rookie Seer” → “Prophet” → “God-tier”*).

No real-money bets. No on-chain doxxing of your opinions.  
Just a clean space to test your intuition and build a “forecasting profile”.

---

## Why?

Open on-chain voting and prediction markets are powerful, but they have two big issues:

1. **Everything is public forever**  
   Your wallet and your political / market opinions get linked on-chain.  
   People self-censor or avoid participating.

2. **High friction (and risk) when money is involved**  
   Real USDC bets → regulatory risk, anxiety, and less playful experimentation.

VeilCast tries a different angle:

- **Anonymous votes**: who voted what is hidden; only aggregate results are public.
- **XP instead of money**: the “reward” is reputation and ranks, not profit/loss.
- **Time-locked results**: you can’t see the crowd’s bias before the poll closes  
  → fewer bandwagon and herding effects.

It’s a small lab for **honest forecasting and social fun**, not a trading venue.

---

## Core Concepts

### 1. Anonymous, One-Person-One-Vote

Each poll is **1 person, 1 vote** — but anonymous:

- Membership in a “voting group” is proven via cryptography (Merkle tree + ZK).
- On-chain, the contract only sees:
  - A **commitment hash** during the commit phase.
  - A **nullifier** during reveal to ensure you only vote once.
- No address or identity is tied to a specific choice publicly.

Result:  
You can be brutally honest about your view, without worrying who’s watching.

---

### 2. Commit–Reveal: Time-Locked Polls

Each poll has three phases:

1. **Commit phase**  
   - You choose an option (e.g. *Yes / No*).  
   - Your vote is hashed and recorded as a **commitment**.  
   - Nobody (including you, from the outside) can see the actual tally.

2. **Reveal phase**  
   - After the commit deadline, you (or the relayer) reveal your choice.  
   - The contract checks:
     - The hash matches the original commitment.
     - Your **nullifier** hasn’t been used before (no double voting).
   - Votes are added to the public tally.

3. **Resolve phase**  
   - Once the real-world outcome is known, the poll creator sets the **correct option**.  
   - At that point, XP can be calculated and awarded.

Until the reveal/resolve stage, the “wisdom of the crowd” stays behind the veil —  
perfect for **sports results, macro events, or spicy governance topics**.

---

### 3. XP & Ranks 

VeilCast has a simple XP system:

- When you **predict correctly**, you earn XP.
- XP pushes you through **ranks** that show your long-term forecasting skill.

Example (placeholder) rank ladder:

- 0–49 XP: *Unknown Seer*  
- 50–149 XP: *Rookie Diviner*  
- 150–299 XP: *Rising Oracle*  
- 300–599 XP: *Seasoned Prophet*  
- 600+ XP: *God-tier Visionary* 🔮

The exact numbers and names can evolve, but the core ideas:

- **Rank = reputation**, not buying power.
- Your **opinions are anonymous**, but your **track record is visible**.
- Over time, a profile becomes:  
  *“This person is often right about L2s and Bitcoin cycles, even if we never see who they are.”*

---

## Example Scenarios

### Crypto & Markets

- *“BTC will touch 150k before Dec 31, 2027.”*  
- *“EigenLayer TVL will exceed \$50B by the end of 2026.”*  
- *“Rollup X will flip Rollup Y in daily active users in 2025.”*

### Sports & Culture

- *“Team A will take the championship this season.”*  
- *“Oscar for Best Picture will go to film X.”*

### Governance & Social

- *“DAO proposal #123 will pass with >60% yes.”*  
- *“New L2 will overtake Optimistic rollups in weekly active addresses next year.”*

---

## How a User Flows Through VeilCast

1. **Join / prove membership**  
   - Get added to the voting group (membership proof, ZK-friendly).

2. **Browse Polls**
   - See active prediction polls with:
     - Question
     - Options
     - Commit / reveal / resolve timelines

3. **Commit Your Vote**
   - During commit phase, select your choice and click “Commit”.
   - Your commitment is stored; nobody can see what you picked.

4. **Reveal & Watch the Outcome**
   - After commit closes, reveal phase opens.
   - Your vote is revealed and included in the tally.
   - Once the real-world result is known, the poll is resolved.

5. **Earn XP & Level Up**
   - If you were right, you gain XP.
   - Your rank badge updates (e.g. from *Rookie Diviner* to *Rising Oracle*).
   - Over time, your profile becomes a public record of your forecasting skill.

---

## What VeilCast Is **Not**

- ❌ A real-money prediction market  
  - No USDC deposits, no leverage, no on-chain trading.
- ❌ A KYC-heavy, regulated derivatives exchange  
  - It’s meant as a **playground** for signals, not a financial product.
- ❌ A public doxxing machine for your beliefs  
  - The whole design is to **separate identity from individual votes**.

---

## Long-Term Vision (Beyond v1)

VeilCast v1 focuses on:

- Simple binary/multi-choice polls,
- Anonymous 1-person-1-vote,
- XP & ranks for correct predictions.

In the future, it could evolve into:

- **More advanced scoring rules** (proper scoring, probability forecasts).
- **Quadratic voting** for intensity of preference.
- **Topic-based reputation** (e.g. “Macro God”, “AI Prophet”, “DeFi Oracle”).
- **Integration with on-chain governance / DAOs** as a “private signal layer”.

---

## Tech Snapshot (High-Level Only)

Just to know what’s under the hood (without going too dev-y):

- **Smart Contracts**: Ethereum-compatible contracts (Foundry)  
  handle polls, commit–reveal, and tallying.
- **Backend / Relayer**: Rust server  
  generates proofs, talks to the chain, and manages XP & profiles.
- **Frontend**: React + TypeScript  
  for poll discovery, voting UX, and XP/rank visualization.
- **Privacy Building Blocks**:  
  Merkle trees, nullifiers, and zk-friendly flows for anonymous group membership.

---

## Status

> **Day 1 — Infra & Monorepo skeleton**

We’re starting with the core foundations:

- Monorepo structure (contracts + backend + frontend),
- Local dev network, and
- Minimal UI/API wiring.

The goal for the first version:  
**a fully working anonymous prediction flow + XP ladder**,  
even if the UI and scoring rules are still simple.

Stay tuned — the veil will lift soon. 👁‍🗨

---

## Dev Quickstart (Infra)

- Bring everything up with Makefile: `make up` (uses `infra/docker-compose.yml` and `.env.*` files).
- Stop stack: `make down`.
- Services:
  - `backend`: `RPC_URL` from `infra/.env.backend` (default `http://localhost:8545`), port `8000:8000`.
  - `frontend`: `VITE_API_BASE` from `infra/.env.frontend` (default `http://localhost:8000`), port `5173:5173`.
- Foundry-only image (no auto chain):
  - Build: `docker build -t veilcast-foundry contracts`
  - Local tests: `docker run --rm -it -v $(pwd)/contracts:/app veilcast-foundry forge test`
  - Testnet deploy: `docker run --rm -it -v $(pwd)/contracts:/app -e RPC_URL=$RPC_URL -e PRIVATE_KEY=$PK veilcast-foundry forge script <script> --rpc-url $RPC_URL --private-key $PRIVATE_KEY --broadcast`
- Hot reload: compose mounts `../backend` and `../frontend`, so code changes reflect inside containers.
- Logs: `cd infra && docker compose logs -f backend` (or `frontend`).
