import { ArrowUpRight, Clock3, Info, Trophy } from 'lucide-react';
import { PollView } from '../lib/types';
import { clsx } from 'clsx';
import { Link } from 'react-router-dom';

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
    <div key={`${opt}-${idx}`} className="rounded-xl bg-white/5 px-3 py-2 text-sm text-white/80">
      {opt}
    </div>
  ));

  const secondaryLabel = (() => {
    if (poll.phase === 'commit') return `Commit ends in ${poll.countdown}`;
    if (poll.phase === 'reveal') {
      return poll.commit_sync_completed ? 'Reveal done, waiting for resolve' : `Reveal ends in ${poll.countdown}`;
    }
    return poll.resolved ? 'Resolved' : 'Resolve pending';
  })();

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
        <span>{secondaryLabel}</span>
      </div>
      <div className="flex flex-col gap-2">{bars}</div>
      <div className="group relative mt-2 flex w-fit items-center gap-2 text-xs text-white/50">
        <Info size={14} className="text-white/70" />
        <span>Membership snapshot: {shorten(poll.membership_root)}</span>
        <div className="pointer-events-none absolute left-0 top-full z-10 hidden w-52 rounded-lg border border-white/10 bg-black/80 p-2 text-[11px] text-white/80 group-hover:flex">
          Captured allowlist root when this poll was created. Proofs must match this snapshot.
        </div>
      </div>
      {poll.resolved && poll.correct_option != null && (
        <div className="flex items-center gap-2 rounded-lg bg-gradient-to-r from-poseidon/30 to-magenta/30 px-3 py-2 text-sm text-white">
          <Trophy size={16} />
          Correct: {poll.options[poll.correct_option] ?? `Option ${poll.correct_option}`}
        </div>
      )}
      <div className="flex items-center justify-between pt-2">
        <Link
          to={`/poll/${poll.id}`}
          className="rounded-full bg-gradient-to-r from-poseidon to-cyan px-4 py-2 text-sm font-semibold shadow-glow hover:opacity-90"
        >
          {poll.phase === 'commit' ? 'View & commit' : poll.phase === 'reveal' ? 'View & reveal' : 'View results'}
        </Link>
        <Link className="flex items-center gap-1 text-sm text-white/70 hover:text-white" to={`/poll/${poll.id}`}>
          Details <ArrowUpRight size={14} />
        </Link>
      </div>
    </div>
  );
}

function shorten(value: string, len = 6) {
  if (value.length <= len * 2) return value;
  return `${value.slice(0, len)}…${value.slice(-len)}`;
}
