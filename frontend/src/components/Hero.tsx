import { Sparkles } from 'lucide-react';

export function Hero() {
  return (
    <section className="relative overflow-hidden rounded-3xl border border-white/10 bg-gradient-to-br from-poseidon/20 via-ink to-magenta/10 p-8 shadow-glow">
      <div className="flex flex-col gap-4">
        <div className="flex items-center gap-2 text-sm text-cyan">
          <Sparkles size={16} />
          Anonymous prediction layer
        </div>
        <h1 className="text-3xl font-bold leading-tight md:text-4xl">
          Forecast boldly, stay anonymous.
          <br />
          Earn XP and ranks with VeilCast.
        </h1>
        <p className="max-w-2xl text-white/70">
          Commit → Reveal → Resolve. One person, one vote, with zk-friendly flows. Browse hot markets by category and
          join the signal without doxxing your wallet.
        </p>
        <div className="flex gap-3">
          <a
            className="rounded-full bg-gradient-to-r from-poseidon to-magenta px-5 py-3 text-sm font-semibold shadow-glow"
            href="#polls"
          >
            Explore polls
          </a>
          <a
            className="rounded-full border border-white/10 px-5 py-3 text-sm text-white/70 hover:border-cyan/60 hover:text-white"
            href="#how"
          >
            How it works
          </a>
        </div>
      </div>
    </section>
  );
}
