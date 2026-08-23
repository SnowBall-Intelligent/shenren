import { mkdirSync } from 'node:fs'
import { spawn } from 'node:child_process'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const root = path.join(path.dirname(fileURLToPath(import.meta.url)), '../..')
mkdirSync(path.join(root, 'e2e/.tmp/uploads'), { recursive: true })
const cargoTargetDir = path.join(root, 'e2e/.tmp/cargo-target')
const apiUrl = new URL(process.env.E2E_API_URL ?? 'http://127.0.0.1:3000')
const bindHost = apiUrl.hostname.includes(':') ? `[${apiUrl.hostname}]` : apiUrl.hostname
const bindAddr = `${bindHost}:${apiUrl.port || (apiUrl.protocol === 'https:' ? '443' : '80')}`

const child = spawn('cargo', ['run', '--manifest-path', 'backend/Cargo.toml'], {
  cwd: root,
  stdio: 'inherit',
  shell: process.platform === 'win32',
  env: {
    ...process.env,
    CARGO_TARGET_DIR: cargoTargetDir,
    DATABASE_URL: 'sqlite://e2e/.tmp/shenren.db?mode=rwc',
    UPLOADS_DIR: 'e2e/.tmp/uploads',
    BIND_ADDR: bindAddr,
    COOKIE_SECURE: 'false',
    COOKIE_SAMESITE: 'Lax',
    // CI + retries hit /login and /submissions far more than production defaults.
    RATE_LIMIT_LOGIN: '10000',
    RATE_LIMIT_SUBMIT: '10000',
    RATE_LIMIT_HOME: '10000',
    RATE_LIMIT_ADMIN: '10000',
    RATE_LIMIT_UPLOADS: '10000',
  },
})

child.on('exit', (code, signal) => {
  if (signal) process.exit(1)
  process.exit(code ?? 1)
})
