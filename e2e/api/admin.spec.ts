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

test('admin can create a person with a QQ CDN avatar', async ({ request }) => {
  await ensureAdmin(request)
  const qq = '123456789'
  const created = await request.post('/api/admin/persons', {
    multipart: { name: unique('QQ头像'), qq },
  })
  expect(created.status(), await created.text()).toBe(201)
  expect((await created.json()).avatar_url).toBe(
    `https://q2.qlogo.cn/headimg_dl?dst_uin=${qq}&spec=0`,
  )

  const invalid = await request.post('/api/admin/persons', {
    multipart: { name: unique('坏QQ'), qq: '0123' },
  })
  expect(invalid.status()).toBe(400)
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
  const { id } = (await submit.json()) as { id: string }

  const rejected = await request.post(`/api/admin/quotes/${id}/reject`, { data: {} })
  expect(rejected.ok(), await rejected.text()).toBeTruthy()
  expect((await rejected.json()).status).toBe('rejected')

  const publicList = await request.get('/api/quotes?page_size=50')
  const items = (await publicList.json()).items as { id: string }[]
  expect(items.some((item) => item.id === id)).toBeFalsy()
})
