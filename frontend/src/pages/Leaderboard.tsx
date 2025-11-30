const mockLeaders = [
  { name: 'seer.eth', xp: 1280, rank: 'God-tier Visionary' },
  { name: '0x9a...12f3', xp: 980, rank: 'Seasoned Prophet' },
  { name: 'anon.forecaster', xp: 720, rank: 'Rising Oracle' },
  { name: 'oracle_boi', xp: 640, rank: 'Rising Oracle' },
  { name: 'mystic', xp: 420, rank: 'Rookie Diviner' },
];

export function LeaderboardPage() {
  return (
    <div className="flex flex-col gap-6">
      <div>
        <p className="text-sm uppercase tracking-wide text-cyan">XP</p>
        <h2 className="text-3xl font-semibold">Leaderboard</h2>
        <p className="text-white/60">Top predictors by XP. (Mock data until backend XP is ready.)</p>
      </div>
      <div className="glass divide-y divide-white/5 overflow-hidden rounded-2xl">
        {mockLeaders.map((u, i) => (
          <div key={u.name} className="flex items-center gap-3 px-5 py-4">
            <div className="flex h-10 w-10 items-center justify-center rounded-full bg-white/10 text-sm font-semibold">
              #{i + 1}
            </div>
            <div className="flex-1">
              <p className="text-sm font-semibold">{u.name}</p>
              <p className="text-xs text-white/60">{u.rank}</p>
            </div>
            <div className="rounded-full bg-white/5 px-3 py-1 text-sm font-semibold text-cyan">{u.xp} XP</div>
          </div>
        ))}
      </div>
    </div>
  );
}
