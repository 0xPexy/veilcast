import fs from "fs";
import { UltraHonkBackend, Barretenberg } from "@aztec/bb.js";
import path from "path";
import { fileURLToPath } from "url";

/**
 * Generate Verifier.sol from ACIR (zk/target/veilcast.json) using bb.js.
 * Usage:
 *   node scripts/generate_verifier_contract.mjs
 * Prereqs: npm install in contracts/test; zk/target/veilcast.json exists.
 */
async function main() {
  const __dirname = path.dirname(fileURLToPath(import.meta.url));
  const rootDir = path.resolve(__dirname, "../../..");
  const acirPath = path.resolve(rootDir, "zk/target/veilcast.json");
  const outPath = path.resolve(rootDir, "contracts/src/Verifier.sol");

  console.log("Generating Verifier.sol using bb.js (node)...");
  console.log("  acir :", acirPath);
  console.log("  out  :", outPath);

  const circuit = JSON.parse(fs.readFileSync(acirPath, "utf8"));
  const bb = await Barretenberg.new({ threads: 1 });
  const backend = new UltraHonkBackend(circuit.bytecode, { threads: 1, bb });

  // Request size-optimized verifier (to avoid EVM bytecode limit).
  const vk = await backend.getVerificationKey({
    keccak: true,
  });
  const verifierSol = await backend.getSolidityVerifier(vk);

  fs.writeFileSync(outPath, verifierSol);
  await backend.destroy?.();
  await bb.destroy?.();

  console.log("Verifier.sol written to", outPath);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
