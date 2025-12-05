import { useMemo, useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { Hero } from '../components/Hero';
import { CategoryFilter } from '../components/CategoryFilter';
import { PollCard } from '../components/PollCard';
import { fetchPolls } from '../lib/api';
import { PollView } from '../lib/types';

const categories = ['All', 'Crypto', 'Macro', 'Sports', 'Governance', 'General'];
const phaseFilters = [
  { key: 'all', label: 'All' },
  { key: 'commit', label: 'Commit' },
  { key: 'reveal', label: 'Reveal' },
  { key: 'resolved', label: 'Resolved' },
];

export function HomePage() {
  const [activeCat, setActiveCat] = useState('All');
  const [phaseFilter, setPhaseFilter] = useState<'all' | 'commit' | 'reveal' | 'resolved'>('all');
  const { data, isLoading, isError, refetch } = useQuery<PollView[]>({
    queryKey: ['polls'],
    queryFn: fetchPolls,
  });

  const filtered = useMemo(() => {
    if (!data) return [];
    return data.filter((p) => {
      const matchesCategory = activeCat === 'All' ? true : (p.category ?? 'General') === activeCat;
      const matchesPhase = phaseFilter === 'all' ? true : p.phase === phaseFilter;
      return matchesCategory && matchesPhase;
    });
  }, [data, activeCat, phaseFilter]);

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

        <div className="flex flex-wrap items-center gap-2 text-sm text-white/60">
          {phaseFilters.map((filter) => (
            <button
              key={filter.key}
              type="button"
              onClick={() => setPhaseFilter(filter.key as typeof phaseFilter)}
              className={`rounded-full px-3 py-1 ${
                phaseFilter === filter.key
                  ? 'bg-gradient-to-r from-poseidon to-cyan text-white shadow-glow'
                  : 'bg-white/5 text-white/70 hover:bg-white/10'
              }`}
            >
              {filter.label}
            </button>
          ))}
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
