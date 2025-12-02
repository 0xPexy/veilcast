import { Buffer } from 'buffer';
// Polyfill Buffer for libraries (bb.js) that expect Node globals
if (!(window as any).Buffer) {
  (window as any).Buffer = Buffer;
}

import React from 'react';
import ReactDOM from 'react-dom/client';
import './index.css';
import App from './App';

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
