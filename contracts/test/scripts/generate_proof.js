import { Barretenberg, UltraHonkBackend, Fr } from "@aztec/bb.js";
import { Noir } from "@noir-lang/noir_js";
import { ethers } from "ethers";
import fs from "fs";
import path from "path";
import { fileURLToPath } from "url";

async function generateProof() {
  const __dirname = path.dirname(fileURLToPath(import.meta.url));
  const circuitPath = path.resolve(__dirname, "../../../zk/target/veilcast.json");
  const circuit = JSON.parse(fs.readFileSync(circuitPath, "utf8"));
  if (!("param_witnesses" in circuit)) circuit.param_witnesses = [];
  if (!("param_witnesses" in (circuit.abi || {}))) circuit.abi.param_witnesses = [];
  if (!("return_witnesses" in circuit)) circuit.return_witnesses = [];
  if (!("return_witnesses" in (circuit.abi || {}))) circuit.abi.return_witnesses = [];

  // args: pollId choice secret identitySecret (commitment/nullifier are computed here)
  const [pollIdArg, choiceArg, secretArg, identitySecretArg] = process.argv.slice(2).map((v) => BigInt(v));

  const pathBits = Array(20).fill(0n);
  const pathSiblings = Array(20).fill(0n);

  const bb = await Barretenberg.new();
  const perm = async (arr) => {
    const inputs = arr.map((x) => new Fr(x));
    const out = await bb.poseidon2Permutation(inputs);
    return BigInt(out[0].toString());
  };
  const poseidon2Hash1 = async (x) => perm([x, 0n, 0n, 0n]);
  const poseidon2Hash2 = async (a, b) => perm([a, b, 0n, 0n]);

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

  // Silence noisy console logs from bb/noir internals.
  const originalLog = console.log;
  console.log = () => {};

  const noir = new Noir(circuit);
  const { witness } = await noir.execute(input);
  const honk = new UltraHonkBackend(circuit.bytecode, { threads: 1, bb });
  const { proof, publicInputs } = await honk.generateProof(witness, { keccak: true });

  // Return the exact public inputs (commitment, nullifier, poll_id, membership_root).
  const encoded =
    ethers.AbiCoder.defaultAbiCoder().encode(["bytes", "bytes32[]", "uint256"], [proof, publicInputs, membershipRoot]);

  // Restore console.log for downstream consumers.
  console.log = originalLog;
  return encoded;
}

(async () => {
  try {
    const res = await generateProof();
    process.stdout.write(res);
    process.exit(0);
  } catch (err) {
    console.error(err);
    process.exit(1);
  }
})();
