import { expect, request as playwrightRequest, test, type APIRequestContext } from '@playwright/test'
import { createApprovedQuote, createPerson, ensureAdmin } from '../helpers/api'
import { API, unique, WEB } from '../helpers/env'

test.describe.configure({ mode: 'serial' })

type KeyOptions = {
  rate_limit?: number | null
  rate_window_secs?: number | null
  total_quota?: number | null
  concurrency_limit?: number | null
  allowed_ips?: string[]
  allowed_domains?: string[]
}

async function createKey(request: APIRequestContext, options: KeyOptions = {}) {
  const response = await request.post('/api/admin/api-keys', {
    data: {
      name: unique('e2e-key'),
      enabled: true,
      rate_limit: options.rate_limit ?? null,
      rate_window_secs: options.rate_window_secs ?? null,
      total_quota: options.total_quota ?? null,
      concurrency_limit: options.concurrency_limit ?? null,
      allowed_ips: options.allowed_ips ?? [],
      allowed_domains: options.allowed_domains ?? [],
    },
  })
  expect(response.status(), await response.text()).toBe(201)
  const body = await response.json()
  expect(body.key).toMatch(/^srk_[0-9a-f]{64}$/)
  return body as { id: number; key: string; key_prefix: string }
}

async function keyContext(playwright: { request: typeof playwrightRequest }, key: string) {
  return playwright.request.newContext({
    baseURL: API,
    extraHTTPHeaders: { Authorization: `Bearer ${key}`, Origin: WEB },
  })
}

test('v1 quote API requires a key and supports pagination, inclusive time range and random', async ({ request, playwright }) => {
  await ensureAdmin(request)
  const personId = await createPerson(request, unique('外部API'))
  const publishedAt = new Date().toISOString()
  const content = unique('时间范围语录')
  const created = await request.post('/api/admin/quotes', {
    data: { person_id: personId, content, published_at: publishedAt },
  })
  expect(created.status(), await created.text()).toBe(201)
  const key = await createKey(request, { allowed_domains: ['127.0.0.1'] })

  const unauthorized = await playwright.request.newContext({ baseURL: API })
  expect((await unauthorized.get('/api/v1/quotes')).status()).toBe(401)
  await unauthorized.dispose()

  const external = await keyContext(playwright, key.key)
  const page = await external.get(
    `/api/v1/quotes?page=1&page_size=1&from=${encodeURIComponent(publishedAt)}&to=${encodeURIComponent(publishedAt)}`,
  )
  expect(page.ok(), await page.text()).toBeTruthy()
  const pageBody = await page.json()
  expect(pageBody.page).toBe(1)
  expect(pageBody.page_size).toBe(1)
  expect(pageBody.items).toHaveLength(1)
  expect(pageBody.items[0].content).toBe(content)

  const random = await external.get(
    `/api/v1/quotes/random?from=${encodeURIComponent(publishedAt)}&to=${encodeURIComponent(publishedAt)}`,
  )
  expect(random.ok(), await random.text()).toBeTruthy()
  expect((await random.json()).content).toBe(content)

  const reserved = await external.get('/api/v1/quotes?person_id=1')
  expect(reserved.status()).toBe(400)
  expect(await reserved.text()).toContain('已预留')

  const empty = await external.get(
    '/api/v1/quotes/random?from=2099-01-01T00%3A00%3A00Z',
  )
  expect(empty.status()).toBe(404)
  await external.dispose()
})

test('v1 API enforces domain, IP, frequency and persistent total quota limits', async ({ request, playwright }) => {
  await ensureAdmin(request)

  const badDomain = await request.post('/api/admin/api-keys', {
    data: {
      name: unique('坏域名'), enabled: true, rate_limit: null, rate_window_secs: null,
      total_quota: null, concurrency_limit: null, allowed_ips: [],
      allowed_domains: ['https://example.com'],
    },
  })
  expect(badDomain.status()).toBe(400)

  const blockedIpKey = await createKey(request, { allowed_ips: ['192.0.2.1'] })
  const blockedIp = await keyContext(playwright, blockedIpKey.key)
  expect((await blockedIp.get('/api/v1/quotes')).status()).toBe(403)
  await blockedIp.dispose()

  const rateKey = await createKey(request, { rate_limit: 1, rate_window_secs: 60 })
  const rate = await keyContext(playwright, rateKey.key)
  expect((await rate.get('/api/v1/quotes')).status()).toBe(200)
  const rateLimited = await rate.get('/api/v1/quotes')
  expect(rateLimited.status()).toBe(429)
  expect(rateLimited.headers()['retry-after']).toBeTruthy()
  await rate.dispose()

  const quotaKey = await createKey(request, { total_quota: 1 })
  const quota = await keyContext(playwright, quotaKey.key)
  const first = await quota.get('/api/v1/quotes')
  expect(first.status()).toBe(200)
  expect(first.headers()['x-quota-remaining']).toBe('0')
  expect((await quota.get('/api/v1/quotes')).status()).toBe(429)

  await request.post(`/api/admin/api-keys/${quotaKey.id}/reset-usage`, { data: {} })
  expect((await quota.get('/api/v1/quotes')).status()).toBe(200)
  await quota.dispose()
})
