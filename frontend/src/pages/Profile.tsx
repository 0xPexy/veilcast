export function ProfilePage() {
  return (
    <div className="flex flex-col gap-6">
      <div>
        <p className="text-sm uppercase tracking-wide text-cyan">Profile</p>
        <h2 className="text-3xl font-semibold">Your stats</h2>
        <p className="text-white/60">
          XP and win/loss history will appear here once the backend exposes those APIs. For now this page is just a
          placeholder.
        </p>
      </div>

      <div className="glass rounded-2xl p-6 text-sm text-white/70">
        Nothing to show yet. After a few polls resolve we&apos;ll surface your rank, total XP, and recent polls here.
      </div>
    </div>
  );
}
