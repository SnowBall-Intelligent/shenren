import { expect, test } from '@playwright/test'
import { API, unique, WEB } from '../helpers/env'
import { createPerson, ensureAdmin } from '../helpers/api'

test('mutating admin routes from a foreign origin are forbidden', async ({ playwright }) => {
  const bare = await playwright.request.newContext({
    baseURL: API,
    extraHTTPHeaders: { Origin: 'https://evil.example' },
  })
  const res = await bare.post('/api/admin/login', {
    data: { username: 'x', password: 'yyyyyy' },
  })
  expect(res.status()).toBe(403)
  await bare.dispose()
})

test('admin can create a person and an approved quote', async ({ request }) => {
  await ensureAdmin(request)
  const name = unique('后台神人')
  const personId = await createPerson(request, name)
  const created = await request.post('/api/admin/quotes', {
    data: { person_id: personId, content: unique('后台直接添加') },
  })
  expect(created.status(), await created.text()).toBe(201)
  const body = await created.json()
  expect(body.status).toBe('approved')
  expect(body.person.name).toBe(name)
})

test('pending submission can be rejected', async ({ request }) => {
  await ensureAdmin(request)
  const personId = await createPerson(request, unique('待拒'))
  const content = unique('会被驳回')
  const submit = await request.post('/api/submissions', {
    data: { person_id: personId, content },
    headers: { Origin: WEB },
  })
  expect(submit.status()).toBe(201)
  const { id } = (await submit.json()) as { id: number }

  const rejected = await request.post(`/api/admin/quotes/${id}/reject`, { data: {} })
  expect(rejected.ok(), await rejected.text()).toBeTruthy()
  expect((await rejected.json()).status).toBe('rejected')

  const publicList = await request.get('/api/quotes?page_size=50')
  const items = (await publicList.json()).items as { id: number }[]
  expect(items.some((item) => item.id === id)).toBeFalsy()
})
