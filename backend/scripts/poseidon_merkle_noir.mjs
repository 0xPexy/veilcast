// Compute Poseidon2 Merkle root/paths using noir_js (same engine/version as circuit).
// Usage: node scripts/poseidon_merkle_noir.mjs members.json
// Input JSON: { "members": ["0x...", ...], "depth": 20 }
// Output JSON: { "root": "0x...", "paths": { "<identity>": { bits: [...], siblings: [...] } } }

import fs from 'fs';
import path from 'path';
import { createHash } from 'crypto';
import { Noir } from '@noir-lang/noir_js';
import { Barretenberg, Fr, BN254_FR_MODULUS } from '@aztec/bb.js';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const CIRCUIT_PATH = path.resolve(__dirname, '../../zk/target/veilcast.json');
const DEFAULT_DEPTH = 20;
const FIELD_ZERO = '0';

const MOD = BigInt(BN254_FR_MODULUS.toString());
const mod = (x) => {
  const n = typeof x === 'bigint' ? x : BigInt(x);
  return ((n % MOD) + MOD) % MOD;
};

const ZERO_FR = new Fr(0n);
const frToDec = (fr) => BigInt(fr.toString()).toString(10);

async function poseidon2Hash1(bb, x) {
  const inputs = [new Fr(mod(x)), ZERO_FR, ZERO_FR, ZERO_FR];
  const [result] = await bb.poseidon2Permutation(inputs);
  return frToDec(result);
}

async function poseidon2Hash(bb, left, right) {
  const inputs = [new Fr(mod(left)), new Fr(mod(right)), ZERO_FR, ZERO_FR];
  const [result] = await bb.poseidon2Permutation(inputs);
  return frToDec(result);
}

async function buildTree(memberEntries, depth) {
  const bb = await Barretenberg.new(1);

  const leafHashes = await Promise.all(
    memberEntries.map((entry) => poseidon2Hash1(bb, entry.field)),
  );
  const minimalSize = Math.max(1, 1 << Math.ceil(Math.log2(Math.max(1, leafHashes.length))));
  const leaves = [...leafHashes];
  while (leaves.length < minimalSize) leaves.push(FIELD_ZERO);

  const levels = [leaves];
  while (levels[levels.length - 1].length > 1) {
    const prev = levels[levels.length - 1];
    const next = [];
    for (let i = 0; i < prev.length; i += 2) {
      const l = prev[i];
      const r = prev[i + 1] ?? FIELD_ZERO;
      next.push(await poseidon2Hash(bb, l, r));
    }
    levels.push(next);
  }

  while (levels.length < depth + 1) {
    const prevRoot = levels[levels.length - 1][0];
    const extended = await poseidon2Hash(bb, prevRoot, FIELD_ZERO);
    levels.push([extended]);
  }

  const root = levels[depth][0];

  const paths = {};
  for (let i = 0; i < memberEntries.length; i++) {
    let idx = i;
    const bits = [];
    const siblings = [];
    for (let d = 0; d < depth; d++) {
      const level = levels[d];
      const isLeft = idx % 2 === 0;
      const sibIdx = isLeft ? idx + 1 : idx - 1;
      bits.push(isLeft ? '0' : '1');
      siblings.push(level[sibIdx] ?? FIELD_ZERO);
      idx = Math.floor(idx / 2);
      if (level.length === 1) idx = 0;
    }
    paths[memberEntries[i].original] = { bits, siblings };
  }

  await bb.destroy();
  return { root, paths };
}

const normalizeMember = (value) => {
  const raw = value?.toString().trim() ?? '';
  if (!raw) return '0';
  if (/^-?\d+$/.test(raw)) {
    return mod(BigInt(raw)).toString();
  }
  if (/^0x[0-9a-fA-F]+$/.test(raw)) {
    return mod(BigInt(raw)).toString();
  }
  const hash = createHash('sha256').update(raw).digest('hex');
  return mod(BigInt(`0x${hash}`)).toString();
};

async function main() {
  const file = process.argv[2];
  if (!file) {
    console.error('Usage: node scripts/poseidon_merkle_noir.mjs members.json');
    process.exit(1);
  }

  const raw = fs.readFileSync(file, 'utf8');
  const parsed = JSON.parse(raw);
  const memberEntries = (parsed.members || []).map((m) => ({
    original: m.toString(),
    field: normalizeMember(m),
  }));
  const depth = parsed.depth || DEFAULT_DEPTH;

  // Load circuit (optional sanity: ensure we use same version as veilcast)
  if (!fs.existsSync(CIRCUIT_PATH)) {
    console.warn('Circuit file not found, skipping noir load:', CIRCUIT_PATH);
  } else {
    const circuit = JSON.parse(fs.readFileSync(CIRCUIT_PATH, 'utf8'));
    // noir is not directly used here, but loading ensures version alignment
    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    const noir = new Noir(circuit);
  }

  const res = await buildTree(memberEntries, depth);
  console.log(JSON.stringify({ ...res, depth }, null, 2));
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
