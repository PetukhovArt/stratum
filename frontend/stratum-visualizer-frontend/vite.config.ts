import { defineConfig } from 'vite'
import solid from 'vite-plugin-solid'

// Default to 18080 — `stratum-lint visualize` traditionally binds 8080, but
// on a typical Windows dev box port 8080 is squatted by EnterpriseDB / WAMP /
// Jenkins / etc., so the project ships with 18080 as the documented backend
// port. Override with `VITE_API_PORT=8080` if you intentionally bind there.
const apiPort = process.env.VITE_API_PORT ?? '18080'

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
