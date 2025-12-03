import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

// Vite config tuned for containerized dev
export default defineConfig({
  plugins: [
    react(),
    {
      name: 'wasm-mime',
      configureServer(server) {
        server.middlewares.use((req, res, next) => {
          if (req.url && req.url.endsWith('.wasm')) {
            res.setHeader('Content-Type', 'application/wasm');
          }
          next();
        });
      },
    },
  ],
  envPrefix: ['VITE_'],
  esbuild: {
    target: 'esnext',
  },
  build: {
    target: 'esnext',
  },
  optimizeDeps: {
    // Skip heavy ZK libs/workers from pre-bundling to avoid missing worker files
    exclude: ['@aztec/bb.js', '@noir-lang/noir_js', 'pino'],
    esbuildOptions: {
      target: 'esnext',
    },
  },
  resolve: {
    alias: {
      pino: '/src/shims/pino-browser.ts',
    },
  },
  server: {
    host: '0.0.0.0',
    port: 5173,
  },
});
