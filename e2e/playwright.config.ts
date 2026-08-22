import { defineConfig, devices } from '@playwright/test'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { API, WEB } from './helpers/env'

const root = path.join(path.dirname(fileURLToPath(import.meta.url)), '..')

export default defineConfig({
  globalSetup: './global-setup.ts',
  timeout: 60_000,
  expect: { timeout: 15_000 },
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  workers: process.env.CI ? 1 : undefined,
  reporter: process.env.CI ? [['github'], ['html', { open: 'never' }]] : 'list',
  webServer: [
    {
      command: 'node e2e/scripts/start-api.mjs',
      cwd: root,
      url: `${API}/api/site`,
      timeout: 180_000,
      reuseExistingServer: !process.env.CI,
    },
    ...(process.env.CI
      ? [
          {
            command: 'npx vite --host 127.0.0.1 --port 5173 --strictPort',
            cwd: path.join(root, 'frontend'),
            url: WEB,
            timeout: 180_000,
            reuseExistingServer: false,
            stdout: 'pipe' as const,
            stderr: 'pipe' as const,
          },
        ]
      : []),
  ],
  projects: [
    {
      name: 'api',
      testDir: './api',
      use: {
        baseURL: API,
        extraHTTPHeaders: { Origin: WEB },
      },
    },
    ...(process.env.CI
      ? [
          {
            name: 'web',
            testDir: './web',
            use: {
              ...devices['Desktop Chrome'],
              baseURL: WEB,
            },
          },
        ]
      : []),
  ],
})
