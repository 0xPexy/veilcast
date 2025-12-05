import { useQuery } from '@tanstack/react-query';
import { fetchLeaderboard } from '../lib/api';
import { UserStats } from '../lib/types';

export function LeaderboardPage() {
  const { data, isLoading, isError, refetch } = useQuery<UserStats[]>({
    queryKey: ['leaderboard'],
    queryFn: () => fetchLeaderboard(),
  });

  return (
    <div className="flex flex-col gap-6">
      <div>
        <p className="text-sm uppercase tracking-wide text-cyan">XP</p>
        <h2 className="text-3xl font-semibold">Leaderboard</h2>
        <p className="text-white/60">
          Top anonymized predictors ranked by XP. Answer correctly to climb tiers like Apprentice → Bronze Adept → Gold
          Seer → Mythic Prophet.
        </p>
      </div>

      {isLoading && <div className="glass rounded-2xl p-6 text-sm text-white/60">Loading leaderboard…</div>}
      {isError && (
        <div className="glass flex items-center justify-between rounded-2xl p-6 text-sm text-amber-300">
          <span>Failed to load leaderboard.</span>
          <button className="text-cyan underline" onClick={() => refetch()}>
            Retry
          </button>
        </div>
      )}

      {data && data.length > 0 && (
        <div className="glass overflow-hidden rounded-2xl">
          <table className="w-full text-sm text-white/80">
            <thead className="bg-white/5 text-xs uppercase tracking-wide text-white/50">
              <tr>
                <th className="px-4 py-3 text-left">Rank</th>
                <th className="px-4 py-3 text-left">User</th>
                <th className="px-4 py-3 text-left">Tier</th>
                <th className="px-4 py-3 text-right">XP</th>
                <th className="px-4 py-3 text-right">Accuracy</th>
              </tr>
            </thead>
            <tbody>
              {data.map((entry) => (
                <tr key={`${entry.username}-${entry.rank ?? entry.username}`} className="border-t border-white/10">
                  <td className="px-4 py-3 font-semibold text-white">
                    {entry.rank ? `#${entry.rank}` : '—'}
                  </td>
                  <td className="px-4 py-3">{entry.username}</td>
                  <td className="px-4 py-3">{entry.tier}</td>
                  <td className="px-4 py-3 text-right">{entry.xp.toLocaleString()} XP</td>
                  <td className="px-4 py-3 text-right">
                    {entry.accuracy.toFixed(1)}% ({entry.correct_votes}/{entry.total_votes})
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
      {data && data.length === 0 && !isLoading && (
        <div className="glass rounded-2xl p-6 text-sm text-white/70">No XP data yet. Be the first to resolve a poll!</div>
      )}
    </div>
  );
}
