// Poseidon2 Merkle builder using @aztec/bb.js Poseidon2 permutation.
// Usage:
//   node backend/scripts/poseidon_merkle.mjs members.json
// Input JSON: { "members": ["0x...", "0x...", ...], "depth": 20 }
// Output JSON: { "root": "0x...", "paths": { "<identity_secret>": { "bits": [...], "siblings": [...] } }, "depth": 20 }

import fs from 'fs';
import { Fr, Barretenberg } from '@aztec/bb.js';

const DEFAULT_DEPTH = 20;
const FIELD_ZERO_FR = new Fr(0n);
const FIELD_ZERO_STR = FIELD_ZERO_FR.toString();
const MODULUS =
  21888242871839275222246405745257275088548364400416034343698204186575808495617n;

const toSafeStr = (v) => (typeof v === 'string' ? v : v.toString());

function toFr(val) {
  if (val instanceof Fr) return val;
  if (typeof val === 'string' && val.startsWith('0x')) val = BigInt(val);
  else if (typeof val === 'string') val = BigInt(val);
  else if (typeof val === 'number') val = BigInt(val);

  const reduced = ((val % MODULUS) + MODULUS) % MODULUS;
  return new Fr(reduced);
}

async function poseidon2Hash2(bb, a, b) {
  return (await bb.poseidon2Hash([toFr(a), toFr(b), FIELD_ZERO_FR, FIELD_ZERO_FR])).toString();
}

async function poseidon2Hash1(bb, x) {
  return (await bb.poseidon2Hash([toFr(x), FIELD_ZERO_FR, FIELD_ZERO_FR, FIELD_ZERO_FR])).toString();
}

async function buildTree(members, depth = DEFAULT_DEPTH) {
  const bb = await Barretenberg.new(1);

  // leaf = poseidon2_hash1(identity_secret)
  const leafHashes = await Promise.all(members.map((m) => poseidon2Hash1(bb, m)));
  // minimal power-of-two to cover members
  const minimalSize = Math.max(1, 1 << Math.ceil(Math.log2(Math.max(1, leafHashes.length))));
  const leaves = [...leafHashes];
  while (leaves.length < minimalSize) leaves.push(FIELD_ZERO_STR);

  // build minimal tree
  const levels = [leaves];
  while (levels[levels.length - 1].length > 1) {
    const prev = levels[levels.length - 1];
    const next = [];
    for (let i = 0; i < prev.length; i += 2) {
      const l = prev[i];
      const r = prev[i + 1] ?? FIELD_ZERO_STR;
      next.push(await poseidon2Hash2(bb, l, r));
    }
    levels.push(next);
  }

  // extend to full depth by hashing root with zero; keep single element levels
  while (levels.length < depth + 1) {
    const prevRoot = levels[levels.length - 1][0];
    const extended = await poseidon2Hash2(bb, prevRoot, FIELD_ZERO_STR);
    levels.push([extended]);
  }

  const root = levels[depth][0];

  // paths per original member (identity_secret string)
  const paths = {};
  for (let i = 0; i < members.length; i++) {
    let idx = i;
    const bits = [];
    const siblings = [];

    for (let d = 0; d < depth; d++) {
      if (d < levels.length - 1) {
        const level = levels[d];
        const isLeft = idx % 2 === 0;
        const sibIdx = isLeft ? idx + 1 : idx - 1;
        bits.push(isLeft ? '0' : '1');
        siblings.push(level[sibIdx] ?? FIELD_ZERO_STR);
        idx = Math.floor(idx / 2);
      } else {
        // beyond built depth: sibling is zero, bit=0, stay at root
        bits.push('0');
        siblings.push(FIELD_ZERO_STR);
        idx = 0;
      }
    }
    paths[toSafeStr(members[i])] = { bits, siblings };
  }

  await bb.destroy();
  return { root, paths, depth };
}

async function main() {
  const file = process.argv[2];
  if (!file) {
    console.error('Usage: node backend/scripts/poseidon_merkle.mjs members.json');
    process.exit(1);
  }
  const raw = fs.readFileSync(file, 'utf8');
  const parsed = JSON.parse(raw);
  const members = (parsed.members || []).map(toSafeStr);
  const depth = parsed.depth || DEFAULT_DEPTH;
  const result = await buildTree(members, depth);
  console.log(JSON.stringify(result, null, 2));
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
