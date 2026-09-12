/// <reference types="vitest/config" />
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { defineConfig, loadEnv } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..')

// https://vite.dev/config/
export default defineConfig(({ mode }) => {
  // Load repo-root `.env` (same file as CHEMLAB_BIND for Axum).
  const env = loadEnv(mode, repoRoot, '')
  const host = env.CHEMLAB_VITE_HOST || '127.0.0.1'
  const port = Number(env.CHEMLAB_VITE_PORT || 5179)

  return {
    plugins: [react(), tailwindcss()],
    // Ensure `npm run dev` from apps/web still sees the repo-root env file.
    envDir: repoRoot,
    server: {
      host,
      port,
      strictPort: true,
      proxy: {
        '/api': {
          // Always loopback: Axum is on the same machine as Vite.
          target: 'http://127.0.0.1:3847',
          changeOrigin: true,
        },
      },
    },
    preview: {
      host,
      port,
    },
    test: {
      environment: 'node',
      include: ['src/**/*.test.ts', 'src/**/*.test.tsx'],
      coverage: {
        provider: 'v8',
        reporter: ['text', 'json-summary'],
        all: true,
        include: ['src/**/*.{ts,tsx}'],
        exclude: ['src/**/*.test.ts', 'src/**/*.test.tsx', 'src/generated/**', 'src/main.tsx'],
        thresholds: {
          lines: 60,
        },
      },
    },
  }
})
