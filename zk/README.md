# zk tooling (Noir + Barretenberg)

Dev container for running Noir circuits and barretenberg (`bb`) backend tooling with pinned versions for reproducibility.

## Build
```bash
docker build -t veilcast-zk zk \
  --build-arg NOIR_VERSION=1.0.0-beta.13
```

Args you can override:
- `NOIR_VERSION`: pinned Noir CLI (via noirup). Change to a tagged release you trust.
- `NOIRUP_URL`, `BBUP_URL`: install script URLs; keep to specific commit/branch for reproducibility.

## Usage
```bash
docker run --rm -it -v $(pwd)/zk:/workspace veilcast-zk bash
# inside: nargo --version && bb --help (if bbup succeeded)
```

Notes:
- Image installs `noirup` and `nargo` via `NOIR_VERSION`.
- Installs `bbup` and `bb` from `BBUP_URL`; bbup auto-selects the compatible `bb` version for your Noir toolchain.
- PATH includes `~/.nargo/bin` and `~/.bbup/bin`; entrypoint preserves it for all commands.

## Automation (Makefile)

`zk/Makefile` wraps common nargo tasks inside the Docker image:

- Build image: `cd zk && make build`
- New circuit: `cd zk && make new CIRCUIT=circuits/my_poll`
- Check: `cd zk && make check CIRCUIT=circuits/my_poll`
- Prove / Verify: `cd zk && make prove CIRCUIT=circuits/my_poll` then `make verify CIRCUIT=circuits/my_poll`
- Shell (interactive): `cd zk && make shell`

Artifacts stay in `zk/<circuit>/proofs` and `target`. The image is always rebuilt (cached) before commands to ensure tooling is present.

## Included example circuit

- Path: `zk/src/main.nr`
- Purpose: commit–reveal consistency + group membership for a yes/no poll.
  - Public inputs: `commitment`, `nullifier`, `poll_id`, `membership_root`
  - Private inputs: `choice`, `secret`, `identity_secret`, `path_siblings[20]`, `path_bits[20]`
  - Hash: Poseidon (via `poseidon` dep) — `hash_1` for leaf, `hash_2` for internal nodes/commit/nullifier.
  - Constraints:
    - choice is boolean (0/1)
    - membership: `leaf = hash_1([identity_secret])`; fold Merkle path (bit=0 left, 1 right) → `membership_root`
    - commitment: `commitment == hash_2([choice, secret])`
    - nullifier: `nullifier == hash_2([identity_secret, poll_id])`
  - Tests: `nargo test` covers happy path + invalid choice, wrong commitment, wrong Merkle path, wrong poll_id for nullifier.
