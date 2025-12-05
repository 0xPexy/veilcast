const TOKEN_KEY = 'veilcast_token';
const IDENTITY_KEY = 'veilcast_identity';

export function setToken(token: string) {
  localStorage.setItem(TOKEN_KEY, token);
}

export function getToken(): string | null {
  return localStorage.getItem(TOKEN_KEY);
}

export function getUsernameFromToken(): string | null {
  const token = getToken();
  if (!token) return null;
  return token.startsWith('token:') ? token.slice('token:'.length) : token;
}

export function clearToken() {
  localStorage.removeItem(TOKEN_KEY);
  localStorage.removeItem(IDENTITY_KEY);
}

export function setIdentitySecret(id: string) {
  localStorage.setItem(IDENTITY_KEY, id);
}

export function getIdentitySecret(): string | null {
  return localStorage.getItem(IDENTITY_KEY);
}
