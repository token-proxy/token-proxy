import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import path from 'path'

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      '@components': path.resolve(__dirname, 'src-dashboard/components'),
    },
  },
  server: {
    proxy: {
      '/api': 'http://localhost:3000',
      '/ap': 'http://localhost:3000',
    },
  },
})