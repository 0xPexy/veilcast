import { useState } from 'react';
import { useMutation } from '@tanstack/react-query';

interface FormState {
  question: string;
  category: string;
  optionA: string;
  optionB: string;
  commitMinutes: number;
  revealMinutes: number;
  membershipRoot: string;
}

async function createPollApi(body: any) {
  const base = import.meta.env.VITE_API_BASE || 'http://localhost:8000';
  const res = await fetch(`${base}/polls`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  });
  if (!res.ok) throw new Error('failed to create poll');
  return res.json();
}

export function CreatePollPage() {
  const [form, setForm] = useState<FormState>({
    question: '',
    category: 'General',
    optionA: '',
    optionB: '',
    commitMinutes: 60,
    revealMinutes: 180,
    membershipRoot: '',
  });

  const { mutateAsync, isPending, isSuccess, error } = useMutation({
    mutationFn: createPollApi,
  });

  const onSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    const commitPhaseEnd = new Date(Date.now() + form.commitMinutes * 60 * 1000).toISOString();
    const revealPhaseEnd = new Date(
      Date.now() + (form.commitMinutes + form.revealMinutes) * 60 * 1000,
    ).toISOString();
    const payload = {
      question: form.question,
      options: [form.optionA, form.optionB],
      commit_phase_end: commitPhaseEnd,
      reveal_phase_end: revealPhaseEnd,
      membership_root: form.membershipRoot || '0x0',
    };
    await mutateAsync(payload);
  };

  return (
    <div className="flex flex-col gap-6">
      <div>
        <p className="text-sm uppercase tracking-wide text-cyan">Create</p>
        <h2 className="text-3xl font-semibold">Create a new poll</h2>
        <p className="text-white/60">
          Owner-only in v1. Fill in question, options, timing, and membership root.
        </p>
      </div>

      <form onSubmit={onSubmit} className="glass flex flex-col gap-4 p-6">
        <label className="flex flex-col gap-2">
          <span className="text-sm text-white/70">Question</span>
          <input
            value={form.question}
            onChange={(e) => setForm({ ...form, question: e.target.value })}
            className="rounded-xl border border-white/10 bg-white/5 px-3 py-2 outline-none focus:border-cyan/60"
            required
          />
        </label>

        <div className="grid gap-4 md:grid-cols-2">
          <label className="flex flex-col gap-2">
            <span className="text-sm text-white/70">Option A</span>
            <input
              value={form.optionA}
              onChange={(e) => setForm({ ...form, optionA: e.target.value })}
              className="rounded-xl border border-white/10 bg-white/5 px-3 py-2 outline-none focus:border-cyan/60"
              required
            />
          </label>
          <label className="flex flex-col gap-2">
            <span className="text-sm text-white/70">Option B</span>
            <input
              value={form.optionB}
              onChange={(e) => setForm({ ...form, optionB: e.target.value })}
              className="rounded-xl border border-white/10 bg-white/5 px-3 py-2 outline-none focus:border-cyan/60"
              required
            />
          </label>
        </div>

        <div className="grid gap-4 md:grid-cols-2">
          <label className="flex flex-col gap-2">
            <span className="text-sm text-white/70">Commit phase (minutes from now)</span>
            <input
              type="number"
              min={1}
              value={form.commitMinutes}
              onChange={(e) => setForm({ ...form, commitMinutes: Number(e.target.value) })}
              className="rounded-xl border border-white/10 bg-white/5 px-3 py-2 outline-none focus:border-cyan/60"
            />
          </label>
          <label className="flex flex-col gap-2">
            <span className="text-sm text-white/70">Reveal phase length (minutes)</span>
            <input
              type="number"
              min={1}
              value={form.revealMinutes}
              onChange={(e) => setForm({ ...form, revealMinutes: Number(e.target.value) })}
              className="rounded-xl border border-white/10 bg-white/5 px-3 py-2 outline-none focus:border-cyan/60"
            />
          </label>
        </div>

        <label className="flex flex-col gap-2">
          <span className="text-sm text-white/70">Membership root (Merkle root)</span>
          <input
            value={form.membershipRoot}
            onChange={(e) => setForm({ ...form, membershipRoot: e.target.value })}
            placeholder="0x..."
            className="rounded-xl border border-white/10 bg-white/5 px-3 py-2 outline-none focus:border-cyan/60"
          />
        </label>

        <div className="flex items-center gap-3">
          <button
            type="submit"
            disabled={isPending}
            className="rounded-full bg-gradient-to-r from-poseidon to-cyan px-5 py-2 text-sm font-semibold shadow-glow disabled:opacity-60"
          >
            {isPending ? 'Creating…' : 'Create poll'}
          </button>
          {isSuccess && <span className="text-sm text-cyan">Poll created!</span>}
          {error && <span className="text-sm text-amber-300">Failed: {(error as Error).message}</span>}
        </div>
      </form>
    </div>
  );
}
