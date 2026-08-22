import { mkdirSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { request as playwrightRequest } from '@playwright/test'
import { ensureAdmin } from './helpers/api'
import { API, WEB } from './helpers/env'

const authDir = path.join(path.dirname(fileURLToPath(import.meta.url)), '.auth')
const adminAuthFile = path.join(authDir, 'admin.json')

export default async function globalSetup() {
  mkdirSync(authDir, { recursive: true })
  const request = await playwrightRequest.newContext({
    baseURL: API,
    extraHTTPHeaders: { Origin: WEB },
  })
  await ensureAdmin(request)
  await request.storageState({ path: adminAuthFile })
  await request.dispose()
}
