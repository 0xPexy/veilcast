#!/usr/bin/env bash
set -euo pipefail

# Regenerate verifier from an existing vk using bb CLI (keccak transcript).
# Prereqs: `bb` binary available in PATH (bb --version), zk/target/vk exists.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
VK_PATH="${ROOT_DIR}/zk/target/vk"
OUT_PATH="${ROOT_DIR}/contracts/src/Verifier.sol"

if ! command -v bb >/dev/null 2>&1; then
  echo "bb CLI not found. Install barretenberg (bb) first." >&2
  exit 1
fi

echo "Generating Verifier.sol using bb write_solidity_verifier..."
echo "  vk   : ${VK_PATH}"
echo "  out  : ${OUT_PATH}"

bb write_solidity_verifier -k "${VK_PATH}" -o "${OUT_PATH}" --oracle_hash keccak

echo "Done. Verifier.sol written to ${OUT_PATH}"
