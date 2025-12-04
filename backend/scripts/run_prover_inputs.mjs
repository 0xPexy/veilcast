// Utility to execute Noir circuit with the inputs declared in zk/Prover.toml
// Usage: node backend/scripts/run_prover_inputs.mjs

import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
import { Noir } from '@noir-lang/noir_js';
import toml from 'toml';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const CIRCUIT_PATH = path.resolve(__dirname, '../../zk/target/veilcast.json');
const PROVER_PATH = path.resolve(__dirname, '../../zk/Prover.toml');

function loadInputFile(filePath) {
  const ext = path.extname(filePath).toLowerCase();
  const raw = fs.readFileSync(filePath, 'utf8');
  if (ext === '.json') {
    return JSON.parse(raw);
  }
  return toml.parse(raw);
}

async function main() {
  if (!fs.existsSync(CIRCUIT_PATH)) {
    console.error('Circuit file not found:', CIRCUIT_PATH);
    process.exit(1);
  }

  const inputPath = process.argv[2] ? path.resolve(process.argv[2]) : PROVER_PATH;
  if (!fs.existsSync(inputPath)) {
    console.error('Input file not found:', inputPath);
    process.exit(1);
  }

  const circuit = JSON.parse(fs.readFileSync(CIRCUIT_PATH, 'utf8'));
  const noir = new Noir(circuit);
  const data = loadInputFile(inputPath);
  const input = {
    commitment: data.commitment,
    nullifier: data.nullifier,
    poll_id: data.poll_id,
    membership_root: data.membership_root,
    choice: data.choice,
    secret: data.secret,
    identity_secret: data.identity_secret,
    path_bits: data.path_bits,
    path_siblings: data.path_siblings,
  };

  try {
    const { witness } = await noir.execute(input);
    console.log('[run_prover_inputs] witness length:', witness.length);
  } catch (err) {
    console.error('[run_prover_inputs] execution failed', err);
    process.exit(1);
  }
}

main();
