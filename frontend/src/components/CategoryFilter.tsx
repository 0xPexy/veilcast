interface Props {
  categories: string[];
  active: string;
  onChange: (cat: string) => void;
}

export function CategoryFilter({ categories, active, onChange }: Props) {
  return (
    <div className="flex flex-wrap gap-2">
      {categories.map((cat) => (
        <button
          key={cat}
          onClick={() => onChange(cat)}
          className={`rounded-full border px-3 py-1 text-sm transition ${
            active === cat
              ? 'border-cyan/70 bg-cyan/10 text-white'
              : 'border-white/10 bg-white/5 text-white/70 hover:border-cyan/40 hover:text-white'
          }`}
        >
          {cat}
        </button>
      ))}
    </div>
  );
}
