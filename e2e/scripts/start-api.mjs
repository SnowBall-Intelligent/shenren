import { mkdirSync } from 'node:fs'
import { spawn } from 'node:child_process'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const root = path.join(path.dirname(fileURLToPath(import.meta.url)), '../..')
mkdirSync(path.join(root, 'e2e/.tmp/uploads'), { recursive: true })

const child = spawn('cargo', ['run', '--manifest-path', 'backend/Cargo.toml'], {
  cwd: root,
  stdio: 'inherit',
  shell: process.platform === 'win32',
  env: {
    ...process.env,
    DATABASE_URL: 'sqlite://e2e/.tmp/shenren.db?mode=rwc',
    UPLOADS_DIR: 'e2e/.tmp/uploads',
    BIND_ADDR: '127.0.0.1:3000',
    COOKIE_SECURE: 'false',
    COOKIE_SAMESITE: 'Lax',
  },
})

child.on('exit', (code, signal) => {
  if (signal) process.exit(1)
  process.exit(code ?? 1)
})
