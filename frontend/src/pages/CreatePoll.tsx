import { useState } from 'react';
import { useMutation } from '@tanstack/react-query';
import { useNavigate } from 'react-router-dom';
import { createPoll } from '../lib/api';

interface FormState {
  question: string;
  category: string;
  optionA: string;
  optionB: string;
  commitMinutes: number;
  revealMinutes: number;
}

const pollCategories = ['General', 'Crypto', 'Macro', 'Sports', 'Governance', 'Culture'];

export function CreatePollPage() {
  const navigate = useNavigate();
  const [form, setForm] = useState<FormState>({
    question: '',
    category: 'General',
    optionA: '',
    optionB: '',
    commitMinutes: 60,
    revealMinutes: 180,
  });
  const [txLink, setTxLink] = useState<string | null>(null);
  const [lastPollId, setLastPollId] = useState<number | null>(null);
  const ETHERSCAN_BASE = import.meta.env.VITE_ETHERSCAN_BASE || 'https://sepolia.etherscan.io';

  const { mutateAsync, isPending, isSuccess, error } = useMutation({ mutationFn: createPoll });

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
      category: form.category,
    };
    setTxLink(null);
    const result = await mutateAsync(payload);
    setLastPollId(result.poll.id);
    if (result.tx_hash) {
      const href = `${ETHERSCAN_BASE}/tx/${result.tx_hash}`;
      setTxLink(href);
      window.open(href, '_blank');
    }
  };

  return (
    <div className="flex flex-col gap-6">
      <div>
        <p className="text-sm uppercase tracking-wide text-cyan">Create</p>
        <h2 className="text-3xl font-semibold">Create a new poll</h2>
        <p className="text-white/60">
          Owner-only in v1. Fill in question, options, and timing. Membership root is injected by the backend (latest allowlist).
        </p>
      </div>

      <form onSubmit={onSubmit} className="glass flex flex-col gap-4 p-6">
        <div className="grid gap-4 md:grid-cols-2">
          <label className="flex flex-col gap-2">
            <span className="text-sm text-white/70">Question</span>
            <input
              value={form.question}
              onChange={(e) => setForm({ ...form, question: e.target.value })}
              className="rounded-xl border border-white/10 bg-white/5 px-3 py-2 outline-none focus:border-cyan/60"
              required
            />
          </label>
          <label className="flex flex-col gap-2">
            <span className="text-sm text-white/70">Category</span>
            <select
              value={form.category}
              onChange={(e) => setForm({ ...form, category: e.target.value })}
              className="rounded-xl border border-white/10 bg-white/5 px-3 py-2 text-white outline-none focus:border-cyan/60"
            >
              {pollCategories.map((cat) => (
                <option key={cat} value={cat}>
                  {cat}
                </option>
              ))}
            </select>
          </label>
        </div>

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

        <div className="flex items-center gap-3">
          <button
            type="submit"
            disabled={isPending}
            className="rounded-full bg-gradient-to-r from-poseidon to-cyan px-5 py-2 text-sm font-semibold shadow-glow disabled:opacity-60"
          >
            {isPending ? 'Creating…' : 'Create poll'}
          </button>
          {isSuccess && (
            <span className="flex items-center gap-3 text-sm text-cyan">
              {lastPollId !== null ? `Poll #${lastPollId} created!` : 'Poll created!'}
              {txLink && (
                <a
                  className="underline decoration-dotted hover:text-white"
                  href={txLink}
                  target="_blank"
                  rel="noreferrer"
                >
                  View tx
                </a>
              )}
              {lastPollId !== null && (
                <button
                  type="button"
                  onClick={() => navigate(`/poll/${lastPollId}`)}
                  className="rounded-full border border-cyan/60 px-3 py-1 text-xs font-semibold text-cyan hover:text-white hover:border-white/70"
                >
                  View poll
                </button>
              )}
            </span>
          )}
          {error && <span className="text-sm text-amber-300">Failed: {(error as Error).message}</span>}
        </div>
      </form>
    </div>
  );
}
