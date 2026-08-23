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

  const listed = await request.get('/api/quotes?page=1&page_size=50')
  expect(listed.ok()).toBeTruthy()
  const page = await listed.json()
  expect(page.items.some((item: { content: string }) => item.content === content)).toBeTruthy()

  const pendingText = unique('待审核投稿')
  const submit = await request.post('/api/submissions', {
    data: { person_id: personId, content: pendingText },
  })
  expect(submit.status(), await submit.text()).toBe(201)

  const after = await request.get('/api/quotes?page=1&page_size=50')
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

test('a proposed person can carry a QQ CDN avatar through approval', async ({ request }) => {
  await ensureAdmin(request)
  const current = await request.get('/api/admin/settings')
  expect(current.ok()).toBeTruthy()
  const settings = await current.json()
  const enabled = await request.put('/api/admin/settings', {
    data: {
      site_name: settings.site_name,
      description: settings.description,
      logo_url: settings.logo_url,
      footer: settings.footer,
      allow_propose_person: true,
    },
  })
  expect(enabled.ok(), await enabled.text()).toBeTruthy()

  const qq = '987654321'
  const submit = await request.post('/api/submissions', {
    data: {
      proposed_person_name: unique('投稿QQ神人'),
      proposed_person_qq: qq,
      content: unique('QQ头像投稿'),
    },
  })
  expect(submit.status(), await submit.text()).toBe(201)
  const { id } = await submit.json()

  const pending = await request.get('/api/admin/quotes?status=pending&page_size=100')
  const item = (await pending.json()).items.find((quote: { id: string }) => quote.id === id)
  expect(item.proposed_person_avatar_url).toBe(
    `https://q2.qlogo.cn/headimg_dl?dst_uin=${qq}&spec=0`,
  )

  const approved = await request.post(`/api/admin/quotes/${id}/approve-json`, { data: {} })
  expect(approved.ok(), await approved.text()).toBeTruthy()
  expect((await approved.json()).person.avatar_url).toBe(
    `https://q2.qlogo.cn/headimg_dl?dst_uin=${qq}&spec=0`,
  )

  const existingId = await createPerson(request, unique('已有神人'))
  const invalid = await request.post('/api/submissions', {
    data: { person_id: existingId, proposed_person_qq: qq, content: unique('无效QQ') },
  })
  expect(invalid.status()).toBe(400)

  await request.put('/api/admin/settings', {
    data: {
      site_name: settings.site_name,
      description: settings.description,
      logo_url: settings.logo_url,
      footer: settings.footer,
      allow_propose_person: settings.allow_propose_person,
    },
  })
})
