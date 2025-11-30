import { Phase, Poll } from './types';

export function computePhase(poll: Poll): Phase {
  const now = Date.now();
  const commitEnd = new Date(poll.commit_phase_end).getTime();
  const revealEnd = new Date(poll.reveal_phase_end).getTime();
  if (poll.resolved || now >= revealEnd) return 'resolved';
  if (now >= commitEnd) return 'reveal';
  return 'commit';
}

export function formatCountdown(target: string): string {
  const now = Date.now();
  const t = new Date(target).getTime();
  const diff = Math.max(0, t - now);
  const mins = Math.floor(diff / 60000);
  const hrs = Math.floor(mins / 60);
  const days = Math.floor(hrs / 24);
  if (days > 0) return `${days}d ${hrs % 24}h`;
  if (hrs > 0) return `${hrs}h ${mins % 60}m`;
  return `${Math.max(mins, 0)}m`;
}
