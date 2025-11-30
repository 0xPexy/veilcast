import { useParams } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';
import { fetchPolls } from '../lib/api';
import { PollView } from '../lib/types';
import { Clock3, ShieldCheck, Trophy } from 'lucide-react';

export function PollDetailPage() {
  const { id } = useParams();
  const pollId = Number(id);
  const { data, isLoading, isError } = useQuery<PollView[]>({ queryKey: ['polls'], queryFn: fetchPolls });
  const poll = data?.find((p) => p.id === pollId);

  if (isLoading) return <div className="glass h-48 animate-pulse bg-white/5" />;
  if (isError || !poll) return <div className="glass p-4 text-amber-300">Poll not found.</div>;

  return (
    <div className="flex flex-col gap-6">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <p className="text-sm uppercase tracking-wide text-cyan">{poll.category ?? 'General'}</p>
          <h2 className="text-3xl font-semibold leading-tight">{poll.question}</h2>
        </div>
        <span className="rounded-full bg-white/10 px-3 py-1 text-sm">
          {poll.phase === 'commit' ? 'Commit' : poll.phase === 'reveal' ? 'Reveal' : 'Resolved'}
        </span>
      </div>

      <div className="grid gap-4 md:grid-cols-2">
        <Card>
          <div className="flex items-center gap-2 text-sm text-white/60">
            <Clock3 size={16} />
            {poll.phase === 'commit'
              ? `Commit ends in ${poll.countdown}`
              : poll.phase === 'reveal'
                ? `Reveal ends in ${poll.countdown}`
                : 'Finished'}
          </div>
          <div className="mt-3 space-y-2">
            {poll.options.map((opt, idx) => (
              <div key={opt} className="flex items-center justify-between rounded-xl bg-white/5 px-3 py-2">
                <span className="text-white/80">{opt}</span>
                <span className="text-white/60">Option {idx}</span>
              </div>
            ))}
          </div>
        </Card>

        <Card>
          <div className="flex items-center gap-2 text-sm text-white/60">
            <ShieldCheck size={16} />
            Membership root
          </div>
          <p className="mt-2 text-sm text-white/70 break-words">{poll.membership_root}</p>
          {poll.resolved && poll.correct_option != null && (
            <div className="mt-3 flex items-center gap-2 rounded-lg bg-gradient-to-r from-poseidon/30 to-magenta/30 px-3 py-2 text-sm text-white">
              <Trophy size={16} />
              Correct: Option {poll.correct_option}
            </div>
          )}
        </Card>
      </div>

      <Card>
        <p className="text-sm text-white/70">Actions</p>
        <div className="mt-3 flex flex-wrap gap-3">
          <button className="rounded-full bg-gradient-to-r from-poseidon to-cyan px-5 py-2 text-sm font-semibold shadow-glow">
            {poll.phase === 'commit' ? 'Commit vote' : poll.phase === 'reveal' ? 'Reveal vote' : 'View results'}
          </button>
          <button className="rounded-full border border-white/10 px-4 py-2 text-sm text-white/70 hover:border-cyan/60 hover:text-white">
            Share
          </button>
        </div>
      </Card>
    </div>
  );
}

function Card({ children }: { children: React.ReactNode }) {
  return <div className="glass p-4">{children}</div>;
}
