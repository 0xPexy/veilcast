import { Barretenberg, UltraHonkBackend, Fr } from "@aztec/bb.js";
import { Noir } from "@noir-lang/noir_js";
import { ethers } from "ethers";
import path from "path";
import fs from "fs";
import { fileURLToPath } from "url";

/**
 * Usage:
 *   node generate_proof.js <pollId> <choice> <secret> <identitySecret>
 * All Merkle path bits/siblings are zero (depth 20). Commitment/nullifier/membership_root
 * are computed inside the script. Proof is generated with keccakZK transcript to match
 * the EVM verifier.
 */
async function generateProof() {
  const resources = [];
  const originalLog = console.log;
  console.log = () => {};
  const __dirname = path.dirname(fileURLToPath(import.meta.url));
  const circuitPath = path.resolve(__dirname, "../../../zk/target/veilcast.json");
  const circuit = JSON.parse(fs.readFileSync(circuitPath, "utf8"));

  const [pollIdArg, choiceArg, secretArg, identitySecretArg] = process.argv.slice(2).map((v) => BigInt(v));

  // Merkle path: zero siblings/bits (depth 20)
  const pathBits = Array(20).fill(0n);
  const pathSiblings = Array(20).fill(0n);

  const bb = await Barretenberg.new();
  resources.push(() => bb.destroy?.());
  const perm = async (arr) => {
    const inputs = arr.map((x) => new Fr(x));
    const out = await bb.poseidon2Permutation(inputs);
    return BigInt(out[0].toString());
  };
  const poseidon2Hash1 = async (x) => perm([x, 0n, 0n, 0n]);
  const poseidon2Hash2 = async (a, b) => perm([a, b, 0n, 0n]);

  // Compute membership root for the zero path
  let node = await poseidon2Hash1(identitySecretArg);
  for (let i = 0; i < 20; i++) {
    node = await poseidon2Hash2(node, pathSiblings[i]);
  }
  const membershipRoot = node;

  const commitment = await poseidon2Hash2(choiceArg, secretArg);
  const nullifier = await poseidon2Hash2(identitySecretArg, pollIdArg);
  const toStr = (v) => v.toString();
  const input = {
    commitment: toStr(commitment),
    nullifier: toStr(nullifier),
    poll_id: toStr(pollIdArg),
    membership_root: toStr(membershipRoot),
    choice: toStr(choiceArg),
    secret: toStr(secretArg),
    identity_secret: toStr(identitySecretArg),
    path_bits: pathBits.map(toStr),
    path_siblings: pathSiblings.map(toStr),
  };

  // Build witness via noir_js
  const noir = new Noir(circuit);
  const { witness } = await noir.execute(input); // compressed witness

  const honk = new UltraHonkBackend(circuit.bytecode, { threads: 1, bb });
  resources.push(() => honk.destroy?.());
  // Use keccak transcript (non-ZK) to align with current verifier proof size expectations.
  const { proof, publicInputs } = await honk.generateProof(witness, { keccak: true });

  const encoded = ethers.AbiCoder.defaultAbiCoder().encode(
    ["bytes", "bytes32[]", "uint256"],
    [proof, publicInputs, membershipRoot]
  );

  process.stdout.write(encoded);
  // Clean up Barretenberg resources if available.
  for (const closer of resources) {
    try {
      await closer();
    } catch {
      /* ignore */
    }
  }
  console.log = originalLog;
  process.exit(0);
}

generateProof().catch((err) => {
  console.error(err);
  process.exit(1);
});
