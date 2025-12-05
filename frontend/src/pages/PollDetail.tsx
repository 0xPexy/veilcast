import { useParams } from 'react-router-dom';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { fetchPoll, commitVote, fetchMembershipStatus, fetchCommitStatus, fetchSecret, resolvePoll } from '../lib/api';
import { PollView } from '../lib/types';
import { Clock3, ShieldCheck, Trophy } from 'lucide-react';
import { useEffect, useState } from 'react';
import { getIdentitySecret, getToken, getUsernameFromToken } from '../lib/auth';
import { generateProofClient } from '../lib/proof';

const ETHERSCAN_BASE = import.meta.env.VITE_ETHERSCAN_BASE || 'https://sepolia.etherscan.io';

export function PollDetailPage() {
  const { id } = useParams();
  const pollId = Number(id);
  const queryClient = useQueryClient();

  const { data: poll, isLoading, isError } = useQuery<PollView>({
    queryKey: ['poll', pollId],
    queryFn: () => fetchPoll(pollId),
  });
  const token = getToken();
  const username = getUsernameFromToken();
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
  const secretQuery = useQuery({
    queryKey: ['secret', pollId, token],
    queryFn: () => fetchSecret(pollId, token as string),
    enabled: !!token && (membership?.is_member ?? false),
  });

  const [choice, setChoice] = useState(0);
  const [commitSecret, setCommitSecret] = useState('');
  const [identitySecret, setIdentitySecret] = useState(storedIdentity ?? '');
  const [showIdentity, setShowIdentity] = useState(false);
  const [showSecret, setShowSecret] = useState(false);
  const [commitComputed, setCommitComputed] = useState<string | null>(null);
  const [proofPhase, setProofPhase] = useState<'idle' | 'proving' | 'submitting' | 'success' | 'error'>('idle');
  const [proofMessage, setProofMessage] = useState<string>('');
  const [proofStep, setProofStep] = useState(0);
  const [resolveOption, setResolveOption] = useState(0);

  const commitMutation = useMutation({
    mutationFn: async () => {
      if (!poll || !identitySecret) throw new Error('missing poll or identity');
      if (!commitSecret) throw new Error('missing server secret');
      if (!membership?.path_bits || !membership.path_siblings) {
        throw new Error('missing membership merkle path');
      }
      setProofPhase('proving');
      setProofStep(1);
      setProofMessage('Generating anonymous proof…');
      setCommitComputed(null);

      const bundle = await generateProofClient(
        choice,
        commitSecret,
        identitySecret,
        poll.id,
        poll.membership_root,
        membership.path_bits,
        membership.path_siblings,
      );
      setProofPhase('submitting');
      setProofStep(2);
      setProofMessage('Submitting commitment…');

      setCommitComputed(bundle.commitment);
      return commitVote(
        pollId,
        {
          choice,
          secret: commitSecret,
          commitment: bundle.commitment,
          nullifier: bundle.nullifier,
          proof: bundle.proof,
          public_inputs: bundle.public_inputs,
        },
        token || undefined,
      );
    },
    onSuccess: () => {
      setProofPhase('success');
      setProofMessage('Vote locked in. Reveal will run automatically in the reveal window.');
      queryClient.invalidateQueries({ queryKey: ['poll', pollId] });
      queryClient.invalidateQueries({ queryKey: ['commitStatus', pollId, token] });
    },
    onError: (err: any) => {
      setProofPhase('error');
      setProofMessage(err?.message || 'Proof or submission failed.');
    },
  });

  const resolveMutation = useMutation({
    mutationFn: async () => {
      if (!token) throw new Error('Login required');
      return resolvePoll(pollId, resolveOption, token);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['poll', pollId] });
      queryClient.invalidateQueries({ queryKey: ['polls'] });
    },
  });

  const canCommit =
    !!token &&
    (membership?.is_member ?? false) &&
    !(commitStatus?.already_committed ?? false) &&
    !!commitSecret;
  const isOwner = !!username && poll?.owner === username;
  const canResolveNow =
    !!poll && poll.commit_sync_completed && poll.phase === 'resolved' && !poll.resolved;
  useEffect(() => {
    if (poll) {
      setResolveOption(0);
    }
  }, [poll?.id]);

  useEffect(() => {
    if (secretQuery.data?.secret) {
      setCommitSecret(secretQuery.data.secret);
    }
  }, [secretQuery.data?.secret]);

  useEffect(() => {
    if (!poll) return;
    const shouldPoll =
      poll.phase === 'reveal' ||
      (poll.phase === 'resolved' && !poll.resolved);
    if (!shouldPoll) return;
    const id = setInterval(() => {
      queryClient.invalidateQueries({ queryKey: ['poll', pollId] });
    }, 4000);
    return () => clearInterval(id);
  }, [poll?.phase, poll?.resolved, pollId, queryClient]);

  if (isLoading) return <div className="glass h-48 animate-pulse bg-white/5" />;
  if (isError || !poll) return <div className="glass p-4 text-amber-300">Poll not found.</div>;

  const normalizedCounts =
    poll.vote_counts && poll.vote_counts.length === poll.options.length
      ? poll.vote_counts
      : Array.from({ length: poll.options.length }, () => 0);
  const totalVotes = normalizedCounts.reduce((sum, n) => sum + n, 0);
  const showResults = poll.commit_sync_completed || poll.resolved;

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
            {poll.options.map((opt, idx) => {
              const count = normalizedCounts[idx] ?? 0;
              const percent = totalVotes > 0 ? Math.round((count / totalVotes) * 100) : 0;
              return (
                <div key={`${opt}-${idx}`} className="rounded-xl bg-white/5 px-3 py-2 text-white/80">
                  <div className="flex items-center justify-between gap-2">
                    <span>{opt}</span>
                    {showResults && (
                      <span className="text-xs text-white/60">
                        {percent}% · {count} vote{count === 1 ? '' : 's'}
                      </span>
                    )}
                  </div>
                  {showResults && (
                    <div className="mt-2 h-2 rounded-full bg-white/10">
                      <div
                        className="h-full rounded-full bg-gradient-to-r from-poseidon to-cyan"
                        style={{ width: `${percent}%` }}
                      />
                    </div>
                  )}
                </div>
              );
            })}
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
              Correct: {poll.options[poll.correct_option] ?? `Option ${poll.correct_option}`}
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
              <div className="flex flex-col gap-1 text-sm text-white/70">
                Vote secret (server-provided)
                <div className="flex items-center gap-2 rounded-xl border border-white/10 bg-white/5 px-3 py-2">
                  <input
                    value={
                      showSecret
                        ? commitSecret || '…'
                        : (commitSecret || '…').replace(/./g, '*')
                    }
                    readOnly
                    className="flex-1 bg-transparent text-white/80 outline-none"
                  />
                  <button
                    type="button"
                    onClick={() => setShowSecret((v) => !v)}
                    className="text-xs text-cyan"
                  >
                    {showSecret ? 'Hide' : 'Show'}
                  </button>
                </div>
                {secretQuery.isLoading && (
                  <span className="text-xs text-white/60">Fetching secret…</span>
                )}
                {secretQuery.error && (
                  <span className="text-xs text-amber-300">
                    Failed to load secret: {(secretQuery.error as Error).message}
                  </span>
                )}
              </div>
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
              onClick={() => commitMutation.mutate()}
              disabled={commitMutation.isLoading || !canCommit || !commitSecret || secretQuery.isLoading}
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
            <ProofProgress phase={proofPhase} message={proofMessage} step={proofStep} />
          </div>
        )}
        {poll.phase === 'reveal' && (
          <div className="mt-3 flex flex-wrap items-center gap-3 text-sm text-white/80">
            <span>
              {poll.commit_sync_completed
                ? 'Reveal done. Waiting for resolve window.'
                : 'Reveal in progress — waiting for relayer batch.'}
            </span>
            {poll.commit_sync_completed && poll.reveal_tx_hash && (
              <ActionLink href={`${ETHERSCAN_BASE}/tx/${poll.reveal_tx_hash}`} label="View reveal tx" />
            )}
          </div>
        )}
        {poll.phase === 'resolved' && (
          <div className="mt-3 flex flex-wrap items-center gap-3 text-sm text-white/80">
            {poll.resolved
              ? 'Poll resolved. See results above.'
              : 'Resolve window open, waiting for owner to publish the outcome.'}
            {poll.reveal_tx_hash && !poll.resolved && (
              <ActionLink href={`${ETHERSCAN_BASE}/tx/${poll.reveal_tx_hash}`} label="View reveal tx" />
            )}
          </div>
        )}
        {isOwner && poll.phase === 'resolved' && !poll.resolved && (
          <div className="mt-4 rounded-xl border border-white/10 bg-white/5 p-3 text-sm text-white/80">
            <p className="font-semibold text-white">Owner controls</p>
            <p className="mt-1 text-xs text-white/60">
              {poll.commit_sync_completed
                ? poll.phase === 'resolved'
                  ? 'Reveal batch finished. You can input the correct option once outcome is known.'
                  : 'Reveal batch finished. Resolve opens when the reveal window closes.'
                : 'Waiting for relayer to finish reveal before resolving.'}
            </p>
            <div className="mt-2 flex flex-col gap-2 md:flex-row md:items-end">
              <label className="flex flex-col gap-1 text-xs uppercase tracking-wide text-white/60">
                Correct option
                <select
                  value={resolveOption}
                  onChange={(e) => setResolveOption(Number(e.target.value))}
                  className="rounded-xl border border-white/10 bg-black/40 px-3 py-2 text-sm text-white outline-none focus:border-cyan/60"
                >
                  {poll.options.map((opt, idx) => (
                    <option key={`${opt}-${idx}`} value={idx}>
                      {opt}
                    </option>
                  ))}
                </select>
              </label>
              <button
                type="button"
                onClick={() => resolveMutation.mutate()}
                disabled={!canResolveNow || resolveMutation.isLoading}
                className="rounded-full bg-gradient-to-r from-poseidon to-cyan px-4 py-2 text-sm font-semibold shadow-glow disabled:opacity-50"
              >
                {resolveMutation.isLoading
                  ? 'Resolving…'
                  : canResolveNow
                    ? 'Resolve poll'
                    : 'Waiting for resolve window'}
              </button>
            </div>
            {resolveMutation.error && (
              <p className="mt-1 text-xs text-amber-300">
                {(resolveMutation.error as Error).message}
              </p>
            )}
          </div>
        )}
      </Card>
    </div>
  );
}

function ProofProgress({
  phase,
  message,
  step,
}: {
  phase: 'idle' | 'proving' | 'submitting' | 'success' | 'error';
  message: string;
  step: number;
}) {
  if (phase === 'idle') return null;
  const steps = [
    { id: 1, label: 'Generate proof' },
    { id: 2, label: 'Submit commitment' },
  ];
  const badge = (state: 'pending' | 'running' | 'done' | 'error') => {
    switch (state) {
      case 'done':
        return 'text-emerald-300';
      case 'running':
        return 'text-cyan';
      case 'error':
        return 'text-amber-300';
      default:
        return 'text-white/40';
    }
  };

  const getState = (id: number): 'pending' | 'running' | 'done' | 'error' => {
    if (phase === 'proving') return id === 1 ? 'running' : 'pending';
    if (phase === 'submitting') return id === 1 ? 'done' : 'running';
    if (phase === 'success') return 'done';
    if (phase === 'error') {
      if (step < id) return 'pending';
      if (step === id) return 'error';
      return 'done';
    }
    return 'pending';
  };

  return (
    <div className="rounded-xl border border-white/10 bg-white/5 p-3 text-sm text-white/80">
      <p className="font-semibold text-white">Proof progress</p>
      <div className="mt-2 space-y-1">
        {steps.map((item) => {
          const state = getState(item.id);
          return (
            <div key={item.id} className={`flex items-center gap-2 ${badge(state)}`}>
              <span className="text-xs uppercase">{state === 'done' ? 'Done' : state === 'running' ? 'Now' : state === 'error' ? 'Error' : 'Waiting'}</span>
              <span className="text-white/80">{item.label}</span>
            </div>
          );
        })}
      </div>
      <p className="mt-2 text-xs text-white/70">{message}</p>
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

function ActionLink({ href, label }: { href: string; label: string }) {
  return (
    <a
      className="flex items-center gap-1 rounded-full border border-white/20 px-3 py-1 text-xs text-cyan hover:bg-white/5"
      href={href}
      target="_blank"
      rel="noreferrer"
    >
      {label}
    </a>
  );
}
