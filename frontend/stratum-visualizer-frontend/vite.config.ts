import { defineConfig } from 'vite'
import solid from 'vite-plugin-solid'

const apiPort = process.env.VITE_API_PORT ?? '8080'

export default defineConfig({
  plugins: [solid()],
  server: {
    proxy: {
      '/api': `http://127.0.0.1:${apiPort}`,
    },
  },
  build: {
    target: 'es2022',
    sourcemap: true,
  },
  test: {
    environment: 'happy-dom',
    setupFiles: ['./src/test/setup.ts'],
    include: ['src/**/*.test.ts'],
  },
})
