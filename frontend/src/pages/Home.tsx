import { useMemo, useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { Hero } from '../components/Hero';
import { CategoryFilter } from '../components/CategoryFilter';
import { PollCard } from '../components/PollCard';
import { fetchPolls } from '../lib/api';
import { PollView } from '../lib/types';

const categories = ['All', 'Crypto', 'Macro', 'Sports', 'Governance', 'General'];

export function HomePage() {
  const [activeCat, setActiveCat] = useState('All');
  const { data, isLoading, isError, refetch } = useQuery<PollView[]>({
    queryKey: ['polls'],
    queryFn: fetchPolls,
  });

  const filtered = useMemo(() => {
    if (!data) return [];
    if (activeCat === 'All') return data;
    return data.filter((p) => (p.category ?? 'General') === activeCat);
  }, [data, activeCat]);

  return (
    <div className="flex flex-col gap-8">
      <Hero />

      <section id="polls" className="flex flex-col gap-4">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div>
            <p className="text-sm uppercase tracking-wide text-cyan">Live forecasts</p>
            <h2 className="text-2xl font-semibold">Browse active and resolved polls</h2>
          </div>
          <CategoryFilter categories={categories} active={activeCat} onChange={setActiveCat} />
        </div>

        <div className="flex items-center gap-3 text-sm text-white/60">
          <div className="rounded-full bg-white/5 px-3 py-1">Commit window</div>
          <div className="rounded-full bg-white/5 px-3 py-1">Reveal window</div>
          <div className="rounded-full bg-white/5 px-3 py-1">Resolved (results only)</div>
        </div>

        {isLoading && <SkeletonGrid />}
        {isError && (
          <div className="glass flex items-center justify-between p-4 text-sm text-amber-300">
            <span>Failed to load polls from the backend.</span>
            <button className="text-cyan underline" onClick={() => refetch()}>
              Retry
            </button>
          </div>
        )}

        <div className="grid gap-4 md:grid-cols-2">
          {filtered.map((poll) => (
            <PollCard poll={poll} key={poll.id} />
          ))}
        </div>
      </section>

      <section id="how" className="glass grid gap-4 p-6 md:grid-cols-3">
        <Step title="Commit" desc="Pick your choice and lock it in privately with a commitment hash." />
        <Step title="Reveal" desc="When reveal opens, provide proof/nullifier and your vote is tallied." />
        <Step title="Resolve" desc="Once resolved, XP is awarded to correct predictors." />
      </section>
    </div>
  );
}

function SkeletonGrid() {
  return (
    <div className="grid gap-4 md:grid-cols-2">
      {Array.from({ length: 4 }).map((_, i) => (
        <div key={i} className="glass h-48 animate-pulse bg-white/5" />
      ))}
    </div>
  );
}

function Step({ title, desc }: { title: string; desc: string }) {
  return (
    <div className="rounded-2xl border border-white/10 bg-white/5 p-4">
      <p className="text-sm uppercase tracking-wide text-cyan">{title}</p>
      <p className="mt-2 text-white/80">{desc}</p>
    </div>
  );
}
