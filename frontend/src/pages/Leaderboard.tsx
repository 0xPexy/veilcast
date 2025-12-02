export function LeaderboardPage() {
  return (
    <div className="flex flex-col gap-6">
      <div>
        <p className="text-sm uppercase tracking-wide text-cyan">XP</p>
        <h2 className="text-3xl font-semibold">Leaderboard</h2>
        <p className="text-white/60">
          XP tracking is coming soon. Once the backend exposes XP totals, this page will list the top anonymized
          predictors.
        </p>
      </div>
      <div className="glass rounded-2xl p-6 text-sm text-white/70">
        No leaderboard data yet. Keep an eye out after the first reveal/resolution cycle.
      </div>
    </div>
  );
}
