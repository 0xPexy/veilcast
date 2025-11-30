import { useAccount, useConnect, useDisconnect } from 'wagmi';
import { injected } from 'wagmi/connectors';
import { Wallet } from 'lucide-react';
import { Link, useLocation } from 'react-router-dom';

export function Navbar() {
  const { address, isConnected } = useAccount();
  const { connect, connectors, isLoading, pendingConnector } = useConnect();
  const { disconnect } = useDisconnect();
  const location = useLocation();
  const navLinks = [
    { href: '/', label: 'Home' },
    { href: '/create', label: 'Create' },
    { href: '/leaderboard', label: 'Leaderboard' },
    { href: '/profile', label: 'Profile' },
  ];

  const onConnect = () => {
    const inj = connectors.find((c) => c.id === injected({}).id) ?? connectors[0];
    if (inj) connect({ connector: inj });
  };

  return (
    <header className="sticky top-0 z-20 border-b border-white/5 bg-ink/80 backdrop-blur-xl">
      <div className="mx-auto flex max-w-6xl items-center justify-between px-6 py-4">
        <div className="flex items-center gap-3">
          <div className="h-9 w-9 rounded-2xl bg-gradient-to-br from-poseidon to-magenta flex items-center justify-center text-sm font-bold">
            VC
          </div>
          <div>
            <Link to="/" className="text-lg font-semibold hover:text-cyan">
              VeilCast
            </Link>
            <p className="text-xs text-white/60">Anonymous forecasting</p>
          </div>
        </div>
        <div className="flex items-center gap-3">
          <nav className="hidden items-center gap-2 md:flex">
            {navLinks.map((l) => (
              <Link
                key={l.href}
                to={l.href}
                className={`rounded-full px-3 py-1 text-sm ${
                  location.pathname === l.href
                    ? 'bg-white/10 text-white'
                    : 'text-white/60 hover:text-white hover:bg-white/5'
                }`}
              >
                {l.label}
              </Link>
            ))}
          </nav>
          {isConnected ? (
            <button
              onClick={() => disconnect()}
              className="flex items-center gap-2 rounded-full bg-gradient-to-r from-poseidon to-magenta px-4 py-2 text-sm font-semibold shadow-glow"
            >
              <Wallet size={16} />
              <span className="truncate max-w-[120px]">
                {address?.slice(0, 6)}…{address?.slice(-4)}
              </span>
            </button>
          ) : (
            <button
              onClick={onConnect}
              disabled={isLoading}
              className="flex items-center gap-2 rounded-full bg-gradient-to-r from-poseidon to-cyan px-4 py-2 text-sm font-semibold shadow-glow disabled:opacity-60"
            >
              <Wallet size={16} />
              {isLoading ? `Connecting${pendingConnector ? '…' : ''}` : 'Connect wallet'}
            </button>
          )}
        </div>
      </div>
    </header>
  );
}
