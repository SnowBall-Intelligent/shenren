import { request as playwrightRequest } from '@playwright/test'
import { ensureAdmin } from './helpers/api'
import { API, WEB } from './helpers/env'

export default async function globalSetup() {
  const request = await playwrightRequest.newContext({
    baseURL: API,
    extraHTTPHeaders: { Origin: WEB },
  })
  await ensureAdmin(request)
  await request.dispose()
}
