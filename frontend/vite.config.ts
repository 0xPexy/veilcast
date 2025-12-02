import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

// Vite config tuned for containerized dev
export default defineConfig({
  plugins: [react()],
  envPrefix: ['VITE_'],
  server: {
    host: '0.0.0.0',
    port: 5173,
  },
});
