import path from 'node:path'
import tailwindcss from '@tailwindcss/vite'
import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'

// https://vite.dev/config/
export default defineConfig({
  plugins: [react(), tailwindcss()],
  server: {
    // Mismo origen para frontend y API en desarrollo: la cookie de refresh (SameSite=Strict)
    // funciona sin CORS. /api/* → backend en 127.0.0.1:8080 (sin el prefijo /api).
    proxy: {
      '/api': {
        target: 'http://127.0.0.1:8080',
        rewrite: (p) => p.replace(/^\/api/, ''),
      },
    },
  },
  resolve: {
    alias: {
      '@': path.resolve(import.meta.dirname, './src'),
    },
  },
})
