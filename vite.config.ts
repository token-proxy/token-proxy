import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import path from 'path';

export default defineConfig({
  plugins: [
    react(),
    // SemiPlugin({theme: '@semi-bot/semi-theme-token-proxy'}),
  ],
  resolve: {
    alias: {
      '@components': path.resolve(__dirname, 'src-dashboard/components'),
    },
  },
  server: {
    host: '0.0.0.0',
    proxy: {
      '/api': 'http://localhost:3000',
      '/ap': 'http://localhost:3000',
    },
  },
});
