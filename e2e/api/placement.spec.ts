import { expect, test } from '@playwright/test'
import { createApprovedQuote, createPerson, ensureAdmin } from '../helpers/api'
import { unique } from '../helpers/env'

test.describe.configure({ mode: 'serial' })

async function quoteIds(
  request: import('@playwright/test').APIRequestContext,
  personId?: number,
) {
  const query = personId ? `?page=1&page_size=50&person_id=${personId}` : '?page=1&page_size=50'
  const res = await request.get(`/api/quotes${query}`)
  expect(res.ok()).toBeTruthy()
  const body = await res.json()
  return (body.items as { id: string; content: string }[]).map((item) => item.id)
}

async function findQuote(
  request: import('@playwright/test').APIRequestContext,
  content: string,
  personId?: number,
) {
  const query = personId ? `?page=1&page_size=50&person_id=${personId}` : '?page=1&page_size=50'
  const res = await request.get(`/api/quotes${query}`)
  const body = await res.json()
  return (body.items as { id: string; content: string }[]).find((item) => item.content === content)
}

test('new quote without anchor appears before older manually placed quotes', async ({ request }) => {
  await ensureAdmin(request)
  const personId = await createPerson(request, unique('链头神人'))
  const oldContent = unique('旧语录已插位')
  const oldId = await createApprovedQuote(request, personId, oldContent)

  const newContent = unique('新语录应在前')
  const created = await request.post('/api/admin/quotes', {
    data: { person_id: personId, content: newContent },
  })
  expect(created.status(), await created.text()).toBe(201)
  const newId = (await created.json()).id as string

  const order = await quoteIds(request, personId)
  expect(order.indexOf(newId)).toBeLessThan(order.indexOf(oldId))
})

test('place_after_id inserts immediately after anchor', async ({ request }) => {
  await ensureAdmin(request)
  const personId = await createPerson(request, unique('锚点后'))
  const first = unique('锚点第一条')
  const second = unique('锚点第二条')
  const firstId = await createApprovedQuote(request, personId, first)
  await createApprovedQuote(request, personId, second)

  const middle = unique('插在中间')
  const created = await request.post('/api/admin/quotes', {
    data: {
      person_id: personId,
      content: middle,
      place_after_id: firstId,
    },
  })
  expect(created.status()).toBe(201)
  const middleId = (await created.json()).id as string

  const order = await quoteIds(request, personId)
  expect(order.indexOf(middleId)).toBe(order.indexOf(firstId) + 1)
  const secondItem = await findQuote(request, second, personId)
  expect(secondItem).toBeTruthy()
  if (secondItem) {
    // Inserting after `first` only splices `middle`; earlier quotes stay above `first`.
    expect(order.indexOf(secondItem.id)).toBeLessThan(order.indexOf(firstId))
    expect(order.indexOf(firstId)).toBeLessThan(order.indexOf(middleId))
  }
})

test('past published_at lands after the just-newer quote', async ({ request }) => {
  await ensureAdmin(request)
  const personId = await createPerson(request, unique('时间插位'))
  const newer = unique('较新语录')
  const newerAt = '2026-06-01T12:00:00.000Z'
  const newerRes = await request.post('/api/admin/quotes', {
    data: { person_id: personId, content: newer, published_at: newerAt },
  })
  expect(newerRes.status(), await newerRes.text()).toBe(201)
  const newerId = (await newerRes.json()).id as string

  const olderContent = unique('较旧语录')
  const past = '2026-01-01T00:00:00.000Z'
  const created = await request.post('/api/admin/quotes', {
    data: {
      person_id: personId,
      content: olderContent,
      published_at: past,
    },
  })
  expect(created.status()).toBe(201)
  const olderId = (await created.json()).id as string

  const order = await quoteIds(request, personId)
  expect(order.indexOf(olderId)).toBe(order.indexOf(newerId) + 1)
})

test('reject invalid placement anchors', async ({ request }) => {
  await ensureAdmin(request)
  const personId = await createPerson(request, unique('非法锚点'))
  const anchorId = await createApprovedQuote(request, personId, unique('合法锚点'))

  const both = await request.post('/api/admin/quotes', {
    data: {
      person_id: personId,
      content: unique('双锚点'),
      place_before_id: anchorId,
      place_after_id: anchorId,
    },
  })
  expect(both.status()).toBe(400)

  const missing = await request.post('/api/admin/quotes', {
    data: {
      person_id: personId,
      content: unique('无锚点'),
      place_before_id: '00000000-0000-0000-0000-000000000099',
    },
  })
  expect(missing.status()).toBe(400)

  const pinned = await request.post('/api/admin/quotes', {
    data: { person_id: personId, content: unique('置顶锚'), pinned: true },
  })
  expect(pinned.status()).toBe(201)
  const pinnedId = (await pinned.json()).id as string

  const mismatch = await request.post('/api/admin/quotes', {
    data: {
      person_id: personId,
      content: unique('置顶不一致'),
      pinned: false,
      place_before_id: pinnedId,
    },
  })
  expect(mismatch.status()).toBe(400)
})

test('deleting a middle quote keeps chain contiguous', async ({ request }) => {
  await ensureAdmin(request)
  const personId = await createPerson(request, unique('删除中间'))
  const a = await createApprovedQuote(request, personId, unique('链A'))
  const b = await createApprovedQuote(request, personId, unique('链B'))
  const c = await createApprovedQuote(request, personId, unique('链C'))

  await request.post('/api/admin/quotes/reorder', {
    data: { ids: [a, b, c] },
  })

  const deleted = await request.delete(`/api/admin/quotes/${b}`)
  expect(deleted.ok(), await deleted.text()).toBeTruthy()

  const order = await quoteIds(request, personId)
  expect(order.indexOf(a)).toBeLessThan(order.indexOf(c))
  expect(order).not.toContain(b)
})
