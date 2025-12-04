import { Buffer } from 'buffer';
import { Noir } from '@noir-lang/noir_js';
import { CompiledCircuit } from '@noir-lang/types';
import { computeCommitment, computeNullifier } from './zk';

// Ensure Buffer exists in browser/workers before loading noir/backends
if (!(globalThis as any).Buffer) {
  (globalThis as any).Buffer = Buffer;
}
// Some dependencies still reference process; provide minimal shim
if (!(globalThis as any).process) {
  (globalThis as any).process = { env: {} };
}

// Force acvm/noirc wasm path to our public asset in dev to avoid wrong MIME/404
const originalFetch = globalThis.fetch;
const ACVM_PATH = '/acvm_js_bg.wasm';
const NOIRC_PATH = '/noirc_abi_wasm_bg.wasm';
globalThis.fetch = (input: RequestInfo | URL, init?: RequestInit) => {
  if (typeof input === 'string' || input instanceof URL) {
    const url = input.toString();
    if (url.includes('acvm_js_bg.wasm')) return originalFetch(ACVM_PATH, init);
    if (url.includes('noirc_abi_wasm_bg.wasm')) return originalFetch(NOIRC_PATH, init);
  }
  return originalFetch(input, init);
};

type Loaded = { circuit: CompiledCircuit; noir: Noir; honk: UltraHonkBackend };

let cached: Loaded | null = null;

async function loadCircuit(): Promise<Loaded> {
  if (cached) return cached;
  const { UltraHonkBackend } = await import('@aztec/bb.js');
  const res = await fetch('/veilcast.json');
  if (!res.ok) throw new Error('failed to load circuit');
  const circuit = (await res.json()) as CompiledCircuit & { bytecode: any };
  const honk = new UltraHonkBackend(circuit.bytecode, { threads: 1 });
  const noir = new Noir(circuit);
  cached = { circuit, noir, honk };
  return cached;
}

const FIELD_MODULUS =
  21888242871839275222246405745257275088548364400416034343698204186575808495617n;

function toBigIntAuto(val: string | number): bigint {
  if (typeof val === 'number') return BigInt(val);
  if (val.startsWith('0x') || val.startsWith('0X')) {
    return BigInt(val);
  }
  return BigInt(val);
}

function toField(val: string | number): bigint {
  const bi = toBigIntAuto(val);
  const m = ((bi % FIELD_MODULUS) + FIELD_MODULUS) % FIELD_MODULUS;
  return m;
}

function hexPad(value: bigint): string {
  return '0x' + value.toString(16);
}

function bytesToHex(bytes: Uint8Array): string {
  return (
    '0x' +
    Array.from(bytes)
      .map((b) => b.toString(16).padStart(2, '0'))
      .join('')
  );
}

export interface GeneratedProof {
  commitment: string;
  nullifier: string;
  proof: string;
  public_inputs: string[];
}

/**
 * Manual helper: run the circuit with the known-good Prover.toml inputs.
 * Call from console: await selfTestProverInputs();
 */
export async function selfTestProverInputs() {
  console.log("selfTestProverInputs()")
  const { noir } = await loadCircuit();
  const zeroPath = Array(20).fill('0');
  const input = {
    commitment: '10768868654894799257910424228206332091878549504228410743539346844038381518440',
    nullifier: '18864455324415923350837963873555520089825199659156710189515894249203574371945',
    poll_id: '0',
    membership_root: '479653171209926143526691675219993872726743078529227032620836821713077361170',
    choice: '1',
    secret: '42',
    identity_secret: '123',
    path_bits: zeroPath,
    path_siblings: zeroPath,
  };
  console.log('[selfTestProver] input built', input);
  try {
    const res = await noir.execute(input);
    console.log('[selfTestProverInputs] witness len', res.witness.length);
  } catch (e) {
    console.error('[selfTestProverInputs] failed', e);
    throw e;
  }
}

export async function generateProofClient(
  choice: number,
  secret: string,
  identitySecret: string,
  pollId: number,
  membershipRoot: string,
  path_bits: string[],
  path_siblings: string[],
): Promise<GeneratedProof> {
  const { noir, honk } = await loadCircuit();

  const fieldChoice = toField(choice);
  const fieldSecret = toField(secret);
  // Use identity exactly as provided (hex from server); ensure string is preserved
  const identityRaw = identitySecret;
  const fieldId = toField(identityRaw);
  const fieldPoll = toField(pollId);
  const fieldRoot = toField(membershipRoot);

  const commitment = await computeCommitment(Number(fieldChoice), fieldSecret.toString());
  const nullifier = await computeNullifier(fieldId.toString(), Number(fieldPoll));

  // normalize path arrays to depth 20
  const bitsPadded = Array.from({ length: 20 }, (_, i) => toField(path_bits[i] ?? 0));
  const sibPadded = Array.from({ length: 20 }, (_, i) => toField(path_siblings[i] ?? 0));

  const input = {
    commitment: toField(commitment).toString(),
    nullifier: toField(nullifier).toString(),
    choice: fieldChoice.toString(),
    secret: fieldSecret.toString(),
    identity_secret: fieldId.toString(),
    poll_id: fieldPoll.toString(),
    membership_root: fieldRoot.toString(),
    path_bits: bitsPadded.map((b) => b.toString()),
    path_siblings: sibPadded.map((s) => s.toString()),
  };

  let witness;
  try {
    const res = await noir.execute(input);
    witness = res.witness;
  } catch (e) {
    throw e;
  }
  const { proof: proofBytes, publicInputs } = await honk.generateProof(witness, { keccak: true });

  const public_inputs = publicInputs.map((v: any) => hexPad(toBigIntAuto(v)));

  return {
    commitment,
    nullifier,
    proof: bytesToHex(proofBytes as Uint8Array),
    public_inputs,
  };
}
