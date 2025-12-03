import { Poll, PollView, CommitStatus, MembershipStatus, CreatePollResult } from './types';
import { computePhase, formatCountdown } from './time';

const API_BASE = import.meta.env.VITE_API_BASE || 'http://localhost:8000';

function withDerived(polls: Poll[]): PollView[] {
  return polls.map((p) => {
    const phase = computePhase(p);
    const countdown =
      phase === 'commit'
        ? formatCountdown(p.commit_phase_end)
        : phase === 'reveal'
          ? formatCountdown(p.reveal_phase_end)
          : 'done';
    return { ...p, phase, countdown };
  });
}

export async function fetchPolls(): Promise<PollView[]> {
  const res = await fetch(`${API_BASE}/polls`);
  if (!res.ok) throw new Error('failed to fetch polls');
  const data = (await res.json()) as Poll[];
  return withDerived(data);
}

export async function fetchPoll(pollId: number): Promise<PollView> {
  const res = await fetch(`${API_BASE}/polls/${pollId}`);
  if (!res.ok) throw new Error('failed to fetch poll');
  const data = (await res.json()) as Poll;
  const derived = withDerived([data])[0];
  return derived;
}

export async function fetchMembershipStatus(pollId: number, token: string): Promise<MembershipStatus> {
  const res = await fetch(`${API_BASE}/polls/${pollId}/membership`, {
    headers: {
      Authorization: `Bearer ${token}`,
    },
  });
  if (!res.ok) throw new Error('failed to fetch membership');
  return res.json() as Promise<MembershipStatus>;
}

export async function fetchCommitStatus(pollId: number, token: string): Promise<CommitStatus> {
  const res = await fetch(`${API_BASE}/polls/${pollId}/commit_status`, {
    headers: {
      Authorization: `Bearer ${token}`,
    },
  });
  if (!res.ok) throw new Error('failed to fetch commit status');
  return res.json() as Promise<CommitStatus>;
}

export async function createPoll(body: {
  question: string;
  options: string[];
  commit_phase_end: string;
  reveal_phase_end: string;
  category: string;
}): Promise<CreatePollResult> {
  const res = await fetch(`${API_BASE}/polls`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  });
  if (!res.ok) throw new Error('failed to create poll');
  return res.json() as Promise<CreatePollResult>;
}

export async function commitVote(
  pollId: number,
  payload: {
    choice: number;
    commitment: string;
    nullifier: string;
    proof: string;
    public_inputs: string[];
  },
  token?: string,
) {
  const res = await fetch(`${API_BASE}/polls/${pollId}/commit`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      ...(token ? { Authorization: `Bearer ${token}` } : {}),
    },
    body: JSON.stringify(payload),
  });
  if (!res.ok) throw new Error('failed to commit');
  return res.json();
}

export async function proveVote(pollId: number, choice: number, secret: string, identitySecret: string) {
  const res = await fetch(`${API_BASE}/polls/${pollId}/prove`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ choice, secret, identity_secret: identitySecret }),
  });
  if (!res.ok) throw new Error('failed to prove');
  return res.json() as Promise<{ proof: string; public_inputs: string[]; commitment: string; nullifier: string }>;
}

export async function revealVote(
  pollId: number,
  payload: { proof: string; public_inputs: string[]; commitment: string; nullifier: string },
) {
  const res = await fetch(`${API_BASE}/polls/${pollId}/reveal`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(payload),
  });
  if (!res.ok) throw new Error('failed to reveal');
  return res.json();
}

export async function login(username: string, password: string) {
  const res = await fetch(`${API_BASE}/auth/login`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ username, password }),
  });
  if (!res.ok) throw new Error('login failed');
  return res.json() as Promise<{ token: string; username: string; identity_secret: string }>;
}

export async function me(token: string) {
  const res = await fetch(`${API_BASE}/auth/me`, {
    headers: {
      Authorization: `Bearer ${token}`,
    },
  });
  if (!res.ok) throw new Error('unauthorized');
  return res.json() as Promise<{ username: string; identity_secret: string }>;
}
