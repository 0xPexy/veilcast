import { Buffer } from 'buffer';

// Ensure Buffer exists in browser/workers before loading bb.js
if (!(globalThis as any).Buffer) {
  (globalThis as any).Buffer = Buffer;
}

let bbModule: typeof import('@aztec/bb.js') | null = null;
let bb: import('@aztec/bb.js').BarretenbergSync | null = null;

// BN254 field modulus
const FIELD_MODULUS =
  21888242871839275222246405745257275088548364400416034343698204186575808495617n;

function modField(x: bigint): bigint {
  const m = ((x % FIELD_MODULUS) + FIELD_MODULUS) % FIELD_MODULUS;
  return m;
}

async function getBarretenberg(): Promise<import('@aztec/bb.js').BarretenbergSync> {
  if (!bbModule) {
    bbModule = await import('@aztec/bb.js');
  }
  if (!bb) {
    bb = await bbModule.BarretenbergSync.initSingleton();
  }
  return bb;
}

function toHex(bytes: Uint8Array): string {
  return (
    '0x' +
    Array.from(bytes)
      .map((b) => b.toString(16).padStart(2, '0'))
      .join('')
  );
}

export async function computeCommitment(choice: number, secret: string | bigint): Promise<string> {
  const api = await getBarretenberg();
  const c = new bbModule!.Fr(modField(BigInt(choice)));
  const s = new bbModule!.Fr(modField(typeof secret === 'string' ? BigInt(secret) : secret));
  const z = new bbModule!.Fr(0n);
  const [result] = api.poseidon2Permutation([c, s, z, z]);
  return toHex(result.toBuffer());
}

export async function computeNullifier(identitySecret: string | bigint, pollId: number): Promise<string> {
  const api = await getBarretenberg();
  const id = new bbModule!.Fr(modField(typeof identitySecret === 'string' ? BigInt(identitySecret) : identitySecret));
  const pid = new bbModule!.Fr(modField(BigInt(pollId)));
  const z = new bbModule!.Fr(0n);
  const [result] = api.poseidon2Permutation([id, pid, z, z]);
  return toHex(result.toBuffer());
}
