import { ArrowUpRight, Clock3, Info, Trophy } from 'lucide-react';
import { PollView } from '../lib/types';
import { clsx } from 'clsx';

interface Props {
  poll: PollView;
}

export function PollCard({ poll }: Props) {
  const phaseStyles: Record<PollView['phase'], string> = {
    commit: 'text-cyan border-cyan/40 bg-cyan/10',
    reveal: 'text-poseidon border-poseidon/50 bg-poseidon/10',
    resolved: 'text-white border-white/20 bg-white/5',
  };

  const bars = poll.options.map((opt, idx) => (
    <div key={opt} className="flex items-center justify-between rounded-xl bg-white/5 px-3 py-2 text-sm">
      <span className="text-white/80">{opt}</span>
      <span className="text-white/60">Option {idx}</span>
    </div>
  ));

  return (
    <div className="glass relative flex flex-col gap-3 p-5">
      <div className="flex items-center justify-between gap-3">
        <span className="rounded-full bg-white/5 px-3 py-1 text-xs uppercase tracking-wide text-white/70">
          {poll.category ?? 'General'}
        </span>
        <span className={clsx('rounded-full border px-3 py-1 text-xs font-semibold', phaseStyles[poll.phase])}>
          {poll.phase === 'commit' ? 'Commit' : poll.phase === 'reveal' ? 'Reveal' : 'Resolved'}
        </span>
      </div>
      <h3 className="text-lg font-semibold leading-tight">{poll.question}</h3>
      <div className="flex items-center gap-2 text-xs text-white/60">
        <Clock3 size={14} />
        <span>
          {poll.phase === 'commit'
            ? `Commit ends in ${poll.countdown}`
            : poll.phase === 'reveal'
              ? `Reveal ends in ${poll.countdown}`
              : 'Finished'}
        </span>
      </div>
      <div className="flex flex-col gap-2">{bars}</div>
      <div className="mt-2 flex items-center gap-2 text-xs text-white/50">
        <Info size={14} />
        <span>Membership root: {shorten(poll.membership_root)}</span>
      </div>
      {poll.resolved && poll.correct_option != null && (
        <div className="flex items-center gap-2 rounded-lg bg-gradient-to-r from-poseidon/30 to-magenta/30 px-3 py-2 text-sm text-white">
          <Trophy size={16} />
          Correct: Option {poll.correct_option}
        </div>
      )}
      <div className="flex items-center justify-between pt-2">
        <button className="rounded-full bg-gradient-to-r from-poseidon to-cyan px-4 py-2 text-sm font-semibold shadow-glow hover:opacity-90">
          {poll.phase === 'commit' ? 'Commit vote' : poll.phase === 'reveal' ? 'Reveal vote' : 'View results'}
        </button>
        <button className="flex items-center gap-1 text-sm text-white/70 hover:text-white">
          Details <ArrowUpRight size={14} />
        </button>
      </div>
    </div>
  );
}

function shorten(value: string, len = 6) {
  if (value.length <= len * 2) return value;
  return `${value.slice(0, len)}…${value.slice(-len)}`;
}
