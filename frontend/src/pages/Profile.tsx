import { useQuery } from '@tanstack/react-query';
import { fetchProfileStats } from '../lib/api';
import { getToken, getUsernameFromToken } from '../lib/auth';
import { UserStats } from '../lib/types';

const TIERS = [
  { name: 'Seedling', min: 0 },
  { name: 'Apprentice', min: 50 },
  { name: 'Bronze Adept', min: 150 },
  { name: 'Silver Sage', min: 350 },
  { name: 'Gold Seer', min: 600 },
  { name: 'Master Oracle', min: 900 },
  { name: 'Mythic Prophet', min: 1500 },
];

export function ProfilePage() {
  const token = getToken();
  const username = getUsernameFromToken();
  const { data, isLoading, isError, refetch } = useQuery<UserStats>({
    queryKey: ['profileStats', token],
    queryFn: () => fetchProfileStats(token as string),
    enabled: !!token,
  });

  if (!token) {
    return (
      <div className="glass rounded-2xl p-6 text-sm text-white/70">
        Login first to view your XP and rank.
      </div>
    );
  }

  const stats = data;
  const currentTier = TIERS.reduce((acc, tier) => (stats && stats.xp >= tier.min ? tier : acc), TIERS[0]);
  const nextTier =
    TIERS.find((tier) => tier.min > (stats?.xp ?? 0)) ?? TIERS[TIERS.length - 1];
  const tierProgress =
    nextTier.min === currentTier.min
      ? 1
      : Math.min(
          1,
          Math.max(0, ((stats?.xp ?? 0) - currentTier.min) / (nextTier.min - currentTier.min)),
        );

  return (
    <div className="flex flex-col gap-6">
      <div>
        <p className="text-sm uppercase tracking-wide text-cyan">Profile</p>
        <h2 className="text-3xl font-semibold">
          {username ? `${username}'s stats` : 'Your stats'}
        </h2>
        <p className="text-white/60">
          XP tracks how often your anonymous predictions were right. Level up by committing early, revealing on time,
          and picking the correct option.
        </p>
      </div>

      {isLoading && <div className="glass rounded-2xl p-6 text-sm text-white/70">Loading stats…</div>}
      {isError && (
        <div className="glass flex items-center justify-between rounded-2xl p-6 text-sm text-amber-300">
          <span>Failed to load profile.</span>
          <button className="text-cyan underline" onClick={() => refetch()}>
            Retry
          </button>
        </div>
      )}

      {stats && (
        <>
          <div className="glass rounded-2xl p-6">
            <div className="flex items-center justify-between">
              <div>
                <p className="text-xs uppercase tracking-wide text-white/60">Current tier</p>
                <h3 className="text-2xl font-semibold text-white">{stats.tier}</h3>
                <p className="text-sm text-white/60">
                  {stats.xp.toLocaleString()} XP ·{' '}
                  {nextTier.min > stats.xp
                    ? `${nextTier.min - stats.xp} XP to ${nextTier.name}`
                    : 'Max tier reached'}
                </p>
              </div>
              <div className="text-right text-sm text-white/60">
                <p>Total votes: {stats.total_votes}</p>
                <p>
                  Accuracy: {stats.accuracy.toFixed(1)}% ({stats.correct_votes}/{stats.total_votes})
                </p>
              </div>
            </div>
            <div className="mt-4 h-3 rounded-full bg-white/10">
              <div
                className="h-full rounded-full bg-gradient-to-r from-poseidon to-cyan transition-all"
                style={{ width: `${tierProgress * 100}%` }}
              />
            </div>
          </div>

          <div className="glass grid gap-4 rounded-2xl p-6 md:grid-cols-2">
            <StatCard title="XP" value={`${stats.xp.toLocaleString()} XP`} />
            <StatCard
              title="Wins"
              value={`${stats.correct_votes}/${stats.total_votes}`}
              subtitle={`${stats.accuracy.toFixed(1)}% accuracy`}
            />
          </div>
        </>
      )}
    </div>
  );
}

function StatCard({ title, value, subtitle }: { title: string; value: string; subtitle?: string }) {
  return (
    <div className="rounded-xl border border-white/10 bg-white/5 p-4">
      <p className="text-xs uppercase tracking-wide text-white/60">{title}</p>
      <p className="text-xl font-semibold text-white">{value}</p>
      {subtitle && <p className="text-sm text-white/60">{subtitle}</p>}
    </div>
  );
}
