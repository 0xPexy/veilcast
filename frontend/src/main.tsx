import React from 'react';
import ReactDOM from 'react-dom/client';
import './index.css';
import App from './App';

// Global polyfills for bb.js / noir
import { Buffer } from 'buffer';
if (!(globalThis as any).Buffer) {
  (globalThis as any).Buffer = Buffer;
}
if (!(globalThis as any).process) {
  (globalThis as any).process = { env: {} };
}

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
