import fs from 'fs';
import path from 'path';
import { UltraHonkBackend, Barretenberg } from '@aztec/bb.js';

async function main() {
  const circuitPath = path.resolve('..','..','zk','target','veilcast.json');
  const circuit = JSON.parse(fs.readFileSync(circuitPath,'utf8'));
  const bb = await Barretenberg.new();
  const backend = new UltraHonkBackend(circuit.bytecode, { threads: 1, bb });
  const verifierSol = await backend.getSolidityVerifier();
  const outPath = path.resolve('..','src','Verifier.sol');
  fs.writeFileSync(outPath, verifierSol);
  console.log('Wrote verifier to', outPath);
  await backend.destroy?.();
  await bb.destroy?.();
  process.exit(0);
}
main().catch((err)=>{console.error(err); process.exit(1);});
