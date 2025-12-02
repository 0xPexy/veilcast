import { Buffer } from 'buffer';

// Ensure Buffer exists in browser/workers before loading bb.js
if (!(globalThis as any).Buffer) {
  (globalThis as any).Buffer = Buffer;
}

let bbModule: typeof import('@aztec/bb.js') | null = null;
let bb: import('@aztec/bb.js').BarretenbergSync | null = null;

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

export async function computeCommitment(choice: number, secret: string): Promise<string> {
  const api = await getBarretenberg();
  const c = new bbModule!.Fr(BigInt(choice));
  const s = new bbModule!.Fr(BigInt(secret));
  const hash = api.poseidon2Hash([c, s]);
  return toHex(hash.toBuffer());
}
