import { Poll, PollView } from './types';
import { computePhase, formatCountdown } from './time';

const API_BASE = import.meta.env.VITE_API_BASE || 'http://localhost:8000';

const fallbackPolls: Poll[] = [
  {
    id: 1,
    question: 'Will BTC reclaim 100k before 2026?',
    options: ['Yes', 'No'],
    commit_phase_end: new Date(Date.now() + 1000 * 60 * 60 * 5).toISOString(),
    reveal_phase_end: new Date(Date.now() + 1000 * 60 * 60 * 24).toISOString(),
    membership_root: '0x0',
    correct_option: null,
    resolved: false,
    category: 'Crypto',
  },
  {
    id: 2,
    question: 'Will Team A win the championship?',
    options: ['Yes', 'No'],
    commit_phase_end: new Date(Date.now() - 1000 * 60 * 30).toISOString(),
    reveal_phase_end: new Date(Date.now() + 1000 * 60 * 60 * 6).toISOString(),
    membership_root: '0x0',
    correct_option: null,
    resolved: false,
    category: 'Sports',
  },
  {
    id: 3,
    question: 'Will Fed cut rates twice this year?',
    options: ['Yes', 'No'],
    commit_phase_end: new Date(Date.now() - 1000 * 60 * 60 * 30).toISOString(),
    reveal_phase_end: new Date(Date.now() - 1000 * 60 * 60 * 2).toISOString(),
    membership_root: '0x0',
    correct_option: 0,
    resolved: true,
    category: 'Macro',
  },
];

function withDerived(polls: Poll[]): PollView[] {
  return polls.map((p) => {
    const phase = computePhase(p);
    const countdown =
      phase === 'commit'
        ? formatCountdown(p.commit_phase_end)
        : phase === 'reveal'
          ? formatCountdown(p.reveal_phase_end)
          : 'done';
    return { ...p, phase, countdown, category: p.category ?? 'General' };
  });
}

export async function fetchPolls(): Promise<PollView[]> {
  try {
    const res = await fetch(`${API_BASE}/polls`);
    if (!res.ok) throw new Error('failed');
    const data = (await res.json()) as Poll[];
    return withDerived(data);
  } catch (err) {
    console.warn('Falling back to local mock polls', err);
    return withDerived(fallbackPolls);
  }
}
