import { useParams } from 'react-router-dom';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { fetchPoll, commitVote, fetchMembershipStatus, fetchCommitStatus } from '../lib/api';
import { PollView } from '../lib/types';
import { Clock3, ShieldCheck, Trophy } from 'lucide-react';
import { useState } from 'react';
import { getIdentitySecret, getToken } from '../lib/auth';
import { generateProofClient, selfTestProverInputs } from '../lib/proof';

export function PollDetailPage() {
  const { id } = useParams();
  const pollId = Number(id);
  const queryClient = useQueryClient();

  const { data: poll, isLoading, isError } = useQuery<PollView>({
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
  const { data: commitStatus, isLoading: commitStatusLoading } = useQuery({
    queryKey: ['commitStatus', pollId, token],
    queryFn: () => fetchCommitStatus(pollId, token as string),
    enabled: !!token,
  });

  const [choice, setChoice] = useState(0);
  const [secret, setSecret] = useState('');
  const [identitySecret, setIdentitySecret] = useState(storedIdentity ?? '');
  const [showIdentity, setShowIdentity] = useState(false);
  const [commitComputed, setCommitComputed] = useState<string | null>(null);
  const [commitError, setCommitError] = useState<string | null>(null);

  const commitMutation = useMutation({
    mutationFn: async () => {
      // Run a dummy proof (Prover.toml inputs) first to ensure Noir execution is healthy.
      await selfTestProverInputs();

      if (!poll || !identitySecret) throw new Error('missing poll or identity');
      if (!membership?.path_bits || !membership.path_siblings) {
        throw new Error('missing membership merkle path');
      }
      console.groupCollapsed('proof inputs');
      console.log('choice', choice);
      console.log('secret', secret);
      console.log('identitySecret', identitySecret);
      console.log('pollId', poll.id);
      console.log('membership_root', poll.membership_root);
      console.log('path_bits', membership.path_bits);
      console.log('path_siblings', membership.path_siblings);
      console.groupEnd();

      const bundle = await generateProofClient(
        choice,
        secret,
        identitySecret,
        poll.id,
        poll.membership_root,
        membership.path_bits,
        membership.path_siblings,
      );
      console.groupCollapsed('generated bundle');
      console.log(bundle);
      console.groupEnd();

      setCommitComputed(bundle.commitment);
      return commitVote(
        pollId,
        {
          choice,
          commitment: bundle.commitment,
          nullifier: bundle.nullifier,
          proof: bundle.proof,
          public_inputs: bundle.public_inputs,
        },
        token || undefined,
      );
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['poll', pollId] });
      queryClient.invalidateQueries({ queryKey: ['commitStatus', pollId, token] });
    },
    onError: (err: any) => {
      setCommitError(err?.message || 'commit failed');
    },
  });

  const canCommit =
    !!token && (membership?.is_member ?? false) && !(commitStatus?.already_committed ?? false);

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
          <p className="mt-2 break-words text-sm text-white/70">{poll.membership_root}</p>
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
                Secret (used for commitment/proof)
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
                  await commitMutation.mutateAsync();
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
                  : 'Lock in vote'}
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
          <div className="mt-3 text-sm text-white/70">
            Reveal in progress. Your commitment will be revealed automatically by the relayer.
          </div>
        )}
        {poll.phase === 'resolved' && (
          <div className="mt-3 text-sm text-white/70">Poll resolved. See results above.</div>
        )}
      </Card>
    </div>
  );
}

function Card({ children }: { children: React.ReactNode }) {
  return (
    <div className="glass flex flex-col gap-3 p-4">
      {children}
    </div>
  );
}
