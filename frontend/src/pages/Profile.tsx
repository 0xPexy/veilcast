const mockHistory = [
  { poll: 'Will BTC reclaim 100k before 2026?', result: 'Win', xp: 50 },
  { poll: 'Will Fed cut rates twice this year?', result: 'Lose', xp: 0 },
  { poll: 'Team A win the championship?', result: 'Pending', xp: 0 },
];

export function ProfilePage() {
  return (
    <div className="flex flex-col gap-6">
      <div>
        <p className="text-sm uppercase tracking-wide text-cyan">Profile</p>
        <h2 className="text-3xl font-semibold">Your stats</h2>
        <p className="text-white/60">Mock profile until backend session/XP API is wired.</p>
      </div>

      <div className="grid gap-4 md:grid-cols-3">
        <StatCard label="Rank" value="Rookie Diviner" />
        <StatCard label="Total XP" value="120" />
        <StatCard label="Win rate" value="62%" />
      </div>

      <div className="glass divide-y divide-white/5 overflow-hidden rounded-2xl">
        {mockHistory.map((h) => (
          <div key={h.poll} className="flex items-center justify-between px-5 py-4">
            <div>
              <p className="text-sm font-semibold">{h.poll}</p>
              <p className="text-xs text-white/60">{h.result}</p>
            </div>
            <div className="text-sm text-cyan">+{h.xp} XP</div>
          </div>
        ))}
      </div>
    </div>
  );
}

function StatCard({ label, value }: { label: string; value: string }) {
  return (
    <div className="glass flex flex-col gap-2 p-4">
      <p className="text-xs uppercase tracking-wide text-white/60">{label}</p>
      <p className="text-xl font-semibold">{value}</p>
    </div>
  );
}
