import { useEffect, useState } from 'react';
import { Link, useLocation, useNavigate } from 'react-router-dom';
import { clearToken, getToken } from '../lib/auth';
import { me } from '../lib/api';

export function Navbar() {
  const [username, setUsername] = useState<string | null>(null);
  const nav = useNavigate();
  const location = useLocation();
  const navLinks = [
    { href: '/', label: 'Home' },
    { href: '/create', label: 'Create' },
    { href: '/leaderboard', label: 'Leaderboard' },
    { href: '/profile', label: 'Profile' },
  ];

  useEffect(() => {
    const refresh = () => {
      const token = getToken();
      if (!token) {
        setUsername(null);
        return;
      }
      me(token)
        .then((res) => setUsername(res.username))
        .catch(() => {
          clearToken();
          setUsername(null);
        });
    };

    refresh();

    const handler = (event: Event) => {
      const detail = (event as CustomEvent<{ username?: string | null }>).detail;
      if (detail && Object.prototype.hasOwnProperty.call(detail, 'username')) {
        setUsername(detail.username ?? null);
      } else {
        refresh();
      }
    };

    window.addEventListener('veilcast-auth-changed', handler as EventListener);
    return () => {
      window.removeEventListener('veilcast-auth-changed', handler as EventListener);
    };
  }, []);

  const onLogout = () => {
    clearToken();
    setUsername(null);
    window.dispatchEvent(new CustomEvent('veilcast-auth-changed', { detail: { username: null } }));
    nav('/login');
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
          {username ? (
            <div className="flex items-center gap-3">
              <div className="rounded-full border border-white/10 bg-white/5 px-4 py-2 text-sm text-white">
                {username}
              </div>
              <button
                onClick={onLogout}
                className="rounded-full bg-gradient-to-r from-poseidon to-magenta px-4 py-2 text-sm font-semibold shadow-glow"
              >
                Logout
              </button>
            </div>
          ) : (
            <Link
              to="/login"
              className="rounded-full bg-gradient-to-r from-poseidon to-cyan px-4 py-2 text-sm font-semibold shadow-glow"
            >
              Sign in
            </Link>
          )}
        </div>
      </div>
    </header>
  );
}
