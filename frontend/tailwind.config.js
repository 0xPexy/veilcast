/** @type {import('tailwindcss').Config} */
export default {
  content: ['./index.html', './src/**/*.{ts,tsx}'],
  theme: {
    extend: {
      colors: {
        ink: '#0b1224',
        cyan: '#4ef0d8',
        poseidon: '#3a6cf6',
        magenta: '#ff5fa2',
        panel: 'rgba(255,255,255,0.05)',
      },
      fontFamily: {
        sans: ['Inter', 'ui-sans-serif', 'system-ui'],
      },
      boxShadow: {
        glow: '0 10px 50px rgba(64, 195, 255, 0.35)',
      },
    },
  },
  plugins: [],
};
