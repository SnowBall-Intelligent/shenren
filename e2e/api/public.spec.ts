import { expect, test } from '@playwright/test'
import { createApprovedQuote, createPerson, ensureAdmin } from '../helpers/api'
import { unique } from '../helpers/env'

test.describe.configure({ mode: 'serial' })

test('GET /api/site returns the default site', async ({ request }) => {
  const res = await request.get('/api/site')
  expect(res.ok()).toBeTruthy()
  const body = await res.json()
  expect(body.site_name).toBe('神人网')
  expect(body.allow_propose_person).toBe(false)
})

test('approved quotes are listed; pending submissions are not', async ({ request }) => {
  await ensureAdmin(request)
  const personName = unique('api神人')
  const content = unique('已通过语录')
  const personId = await createPerson(request, personName)
  await createApprovedQuote(request, personId, content)

  const listed = await request.get('/api/quotes?page=1&page_size=50&recent=true')
  expect(listed.ok()).toBeTruthy()
  const page = await listed.json()
  expect(page.items.some((item: { content: string }) => item.content === content)).toBeTruthy()

  const pendingText = unique('待审核投稿')
  const submit = await request.post('/api/submissions', {
    data: { person_id: personId, content: pendingText },
  })
  expect(submit.status(), await submit.text()).toBe(201)

  const after = await request.get('/api/quotes?page=1&page_size=50&recent=true')
  const afterBody = await after.json()
  expect(afterBody.items.some((item: { content: string }) => item.content === pendingText)).toBeFalsy()

  const review = await request.get('/api/admin/quotes?status=pending')
  expect(review.ok()).toBeTruthy()
  const pending = await review.json()
  expect(pending.items.some((item: { content: string }) => item.content === pendingText)).toBeTruthy()
})

test('quote search matches person or content', async ({ request }) => {
  await ensureAdmin(request)
  const token = unique('搜')
  const personId = await createPerson(request, `${token}名人`)
  await createApprovedQuote(request, personId, `内容里有${token}关键字`)

  const byName = await request.get(`/api/quotes?q=${encodeURIComponent(`${token}名人`)}&recent=true`)
  expect((await byName.json()).total).toBeGreaterThan(0)

  const byContent = await request.get(`/api/quotes?q=${encodeURIComponent(`${token}关键字`)}&recent=true`)
  expect((await byContent.json()).total).toBeGreaterThan(0)

  const miss = await request.get(`/api/quotes?q=${encodeURIComponent(unique('没有这首'))}&recent=true`)
  expect((await miss.json()).total).toBe(0)
})

test('empty submission is rejected', async ({ request }) => {
  await ensureAdmin(request)
  const personId = await createPerson(request, unique('空内容'))
  const res = await request.post('/api/submissions', {
    data: { person_id: personId, content: '   ' },
  })
  expect(res.status()).toBe(400)
})
