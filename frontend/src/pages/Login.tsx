import { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { login, me } from '../lib/api';
import { clearToken, getIdentitySecret, getToken, setIdentitySecret, setToken } from '../lib/auth';

export function LoginPage() {
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [status, setStatus] = useState<'idle' | 'loading' | 'authed'>('idle');
  const nav = useNavigate();

  useEffect(() => {
    const existing = getToken();
    if (existing) {
      me(existing)
        .then((resp) => {
          setIdentitySecret(resp.identity_secret);
          setStatus('authed');
          nav('/');
        })
        .catch(() => {
          clearToken();
        });
    }
  }, [nav]);

  const onSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setStatus('loading');
    try {
      const resp = await login(username, password);
      setToken(resp.token);
      setIdentitySecret(resp.identity_secret);
      window.dispatchEvent(new CustomEvent('veilcast-auth-changed', { detail: { username: resp.username } }));
      setStatus('authed');
      nav('/');
    } catch (err) {
      setStatus('idle');
      setError((err as Error).message);
    }
  };

  return (
    <div className="mx-auto flex max-w-md flex-col gap-6">
      <div>
        <p className="text-sm uppercase tracking-wide text-cyan">Login</p>
        <h2 className="text-3xl font-semibold">Sign in</h2>
        <p className="text-white/60">Demo login accepts any username/password.</p>
      </div>
      <form onSubmit={onSubmit} className="glass flex flex-col gap-4 p-6">
        <label className="flex flex-col gap-2 text-sm text-white/70">
          Username
          <input
            value={username}
            onChange={(e) => setUsername(e.target.value)}
            className="rounded-xl border border-white/10 bg-white/5 px-3 py-2 outline-none focus:border-cyan/60"
            required
          />
        </label>
        <label className="flex flex-col gap-2 text-sm text-white/70">
          Password
          <input
            type="password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            className="rounded-xl border border-white/10 bg-white/5 px-3 py-2 outline-none focus:border-cyan/60"
            required
          />
        </label>
        <button
          type="submit"
          disabled={status === 'loading'}
          className="rounded-full bg-gradient-to-r from-poseidon to-cyan px-5 py-2 text-sm font-semibold shadow-glow disabled:opacity-60"
        >
          {status === 'loading' ? 'Signing in…' : 'Sign in'}
        </button>
        {error && <p className="text-sm text-amber-300">{error}</p>}
      </form>
    </div>
  );
}
