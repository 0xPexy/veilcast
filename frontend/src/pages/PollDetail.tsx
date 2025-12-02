import { useParams } from 'react-router-dom';
import { useQuery, useMutation } from '@tanstack/react-query';
import { fetchPoll, commitVote, proveVote, revealVote, fetchMembershipStatus, fetchCommitStatus } from '../lib/api';
import { PollView } from '../lib/types';
import { Clock3, ShieldCheck, Trophy } from 'lucide-react';
import { useState } from 'react';
import { getIdentitySecret, getToken } from '../lib/auth';
import { computeCommitment } from '../lib/zk';

export function PollDetailPage() {
  const { id } = useParams();
  const pollId = Number(id);
  const { data: poll, isLoading, isError, refetch } = useQuery<PollView>({
    queryKey: ['poll', pollId],
    queryFn: () => fetchPoll(pollId),
  });
  const token = getToken();
  const storedIdentity = getIdentitySecret();
  const { data: membership, isLoading: membershipLoading } = useQuery({
    queryKey: ['membership', pollId, token],
    queryFn: () => fetchMembershipStatus(pollId, token as string),
    enabled: !!token,
  });
  const { data: commitStatus, isLoading: commitStatusLoading, refetch: refetchCommitStatus } = useQuery({
    queryKey: ['commitStatus', pollId, token],
    queryFn: () => fetchCommitStatus(pollId, token as string),
    enabled: !!token,
  });

  const commitMutation = useMutation({
    mutationFn: (commitment: string) => commitVote(pollId, commitment, token || undefined),
    onSuccess: () => {
      refetch();
      refetchCommitStatus();
    },
  });
  const proveMutation = useMutation({
    mutationFn: (input: { choice: number; secret: string; identitySecret: string }) =>
      proveVote(pollId, input.choice, input.secret, input.identitySecret),
  });
  const revealMutation = useMutation({
    mutationFn: (payload: { proof: string; public_inputs: string[]; commitment: string; nullifier: string }) =>
      revealVote(pollId, payload),
    onSuccess: () => refetch(),
  });

  const [commitmentInput, setCommitmentInput] = useState('');
  const [choice, setChoice] = useState(0);
  const [secret, setSecret] = useState('');
  const [identitySecret, setIdentitySecret] = useState(storedIdentity ?? '');
  const [showIdentity, setShowIdentity] = useState(false);
  const [commitComputed, setCommitComputed] = useState<string | null>(null);
  const [commitError, setCommitError] = useState<string | null>(null);

  const canCommit = !!token && (membership?.is_member ?? false) && !(commitStatus?.already_committed ?? false);

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
              <div key={`${opt}-${idx}`} className="rounded-xl bg-white/5 px-3 py-2 text-white/80">
                {opt}
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
        {poll.phase === 'commit' && (
          <div className="mt-3 flex flex-col gap-3">
            <div className="rounded-xl bg-white/5 px-3 py-2 text-sm text-white/70">
              {membershipLoading
                ? 'Checking membership...'
                : !membership?.is_member
                  ? 'Not eligible to vote (login required or not in allowlist).'
                  : commitStatusLoading
                    ? 'Checking commit status...'
                    : commitStatus?.already_committed
                      ? 'Already committed for this poll.'
                      : 'You are eligible to commit.'}
            </div>
            <div className="flex flex-wrap gap-2">
              {poll.options.slice(0, 2).map((opt, idx) => (
                <button
                  key={`${opt}-${idx}`}
                  type="button"
                  onClick={() => setChoice(idx)}
                  className={`rounded-full px-4 py-2 text-sm font-semibold ${
                    choice === idx
                      ? 'bg-gradient-to-r from-poseidon to-cyan text-white shadow-glow'
                      : 'bg-white/5 text-white/80 hover:bg-white/10'
                  }`}
                >
                  {opt}
                </button>
              ))}
            </div>
            <div className="grid gap-2 md:grid-cols-2">
              <label className="flex flex-col gap-1 text-sm text-white/70">
                Secret (keep safe; needed again at reveal)
                <input
                  value={secret}
                  onChange={(e) => setSecret(e.target.value)}
                  className="rounded-xl border border-white/10 bg-white/5 px-3 py-2 outline-none focus:border-cyan/60"
                />
              </label>
              <div className="flex flex-col gap-1 text-sm text-white/70">
                Identity secret (from server)
                <div className="flex items-center gap-2 rounded-xl border border-white/10 bg-white/5 px-3 py-2">
                  <input
                    value={showIdentity ? identitySecret : identitySecret.replace(/./g, '*')}
                    readOnly
                    className="flex-1 bg-transparent text-white/80 outline-none"
                  />
                  <button
                    type="button"
                    onClick={() => setShowIdentity((v) => !v)}
                    className="text-xs text-cyan"
                  >
                    {showIdentity ? 'Hide' : 'Show'}
                  </button>
                </div>
              </div>
            </div>
            <button
              onClick={async () => {
                try {
                  setCommitError(null);
                  const commitment = await computeCommitment(choice, secret || '0');
                  setCommitComputed(commitment);
                  await commitMutation.mutateAsync(commitment);
                } catch (err) {
                  setCommitError((err as Error).message);
                }
              }}
              disabled={commitMutation.isLoading || !canCommit || !secret}
              className="w-fit rounded-full bg-gradient-to-r from-poseidon to-cyan px-5 py-2 text-sm font-semibold shadow-glow disabled:opacity-60"
            >
              {commitMutation.isLoading
                ? 'Committing…'
                : commitStatus?.already_committed
                  ? 'Already committed'
                  : 'Commit vote'}
            </button>
            {commitComputed && (
              <p className="text-xs text-white/60">
                Commitment used: <span className="break-all text-white/80">{commitComputed}</span>
              </p>
            )}
            {commitMutation.error && (
              <p className="text-sm text-amber-300">Failed: {(commitMutation.error as Error).message}</p>
            )}
            {commitError && <p className="text-sm text-amber-300">Error: {commitError}</p>}
          </div>
        )}
        {poll.phase === 'reveal' && (
          <div className="mt-3 flex flex-col gap-3">
            <div className="flex flex-wrap gap-2">
              {poll.options.slice(0, 2).map((opt, idx) => (
                <button
                  key={`${opt}-${idx}`}
                  type="button"
                  onClick={() => setChoice(idx)}
                  className={`rounded-full px-4 py-2 text-sm font-semibold ${
                    choice === idx
                      ? 'bg-gradient-to-r from-poseidon to-cyan text-white shadow-glow'
                      : 'bg-white/5 text-white/80 hover:bg-white/10'
                  }`}
                >
                  {opt}
                </button>
              ))}
            </div>
            <div className="grid gap-2 md:grid-cols-2">
              <label className="flex flex-col gap-1 text-sm text-white/70">
                Secret
                <input
                  value={secret}
                  onChange={(e) => setSecret(e.target.value)}
                  className="rounded-xl border border-white/10 bg-white/5 px-3 py-2 outline-none focus:border-cyan/60"
                />
              </label>
              <label className="flex flex-col gap-1 text-sm text-white/70">
                Identity secret
                <input
                  value={identitySecret}
                  onChange={(e) => setIdentitySecret(e.target.value)}
                  className="rounded-xl border border-white/10 bg-white/5 px-3 py-2 outline-none focus:border-cyan/60"
                />
              </label>
            </div>
            <button
              onClick={async () => {
                const proof = await proveMutation.mutateAsync({
                  choice,
                  secret,
                  identitySecret,
                });
                await revealMutation.mutateAsync(proof);
              }}
              disabled={proveMutation.isLoading || revealMutation.isLoading}
              className="w-fit rounded-full bg-gradient-to-r from-poseidon to-cyan px-5 py-2 text-sm font-semibold shadow-glow disabled:opacity-60"
            >
              {proveMutation.isLoading || revealMutation.isLoading ? 'Revealing…' : 'Generate proof & reveal'}
            </button>
            {(proveMutation.error || revealMutation.error) && (
              <p className="text-sm text-amber-300">
                Failed:{' '}
                {((proveMutation.error ?? revealMutation.error) as Error | undefined)?.message ??
                  'unknown error'}
              </p>
            )}
          </div>
        )}
        {poll.phase === 'resolved' && (
          <p className="mt-3 text-sm text-white/70">Poll resolved. View results above.</p>
        )}
      </Card>
    </div>
  );
}

function Card({ children }: { children: React.ReactNode }) {
  return <div className="glass p-4">{children}</div>;
}
