export const API = process.env.E2E_API_URL ?? 'http://127.0.0.1:3000'
export const WEB = process.env.E2E_WEB_URL ?? 'http://127.0.0.1:5173'
export const ADMIN_USER = process.env.E2E_ADMIN_USER ?? 'e2e-admin'
export const ADMIN_PASS = process.env.E2E_ADMIN_PASS ?? 'e2e-pass-12'

export function unique(prefix: string): string {
  return `${prefix}-${Date.now()}-${Math.floor(Math.random() * 1e6)}`
}
