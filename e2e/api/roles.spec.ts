import { expect, test } from '@playwright/test'
import { ADMIN_PASS, ADMIN_USER, API, unique, WEB } from '../helpers/env'
import { ensureAdmin } from '../helpers/api'

test('admin roles enforce business and super-admin permissions immediately', async ({
  playwright,
  request,
}) => {
  await ensureAdmin(request)

  const meResponse = await request.get('/api/admin/me')
  expect(meResponse.ok(), await meResponse.text()).toBeTruthy()
  const me = (await meResponse.json()) as { id: number; role: string }
  expect(me.role).toBe('super_admin')

  const ordinaryUsername = unique('ordinary-admin')
  const ordinaryPassword = 'ordinary-pass-12'
  const ordinaryCreate = await request.post('/api/admin/admins', {
    data: { username: ordinaryUsername, password: ordinaryPassword },
  })
  expect(ordinaryCreate.status(), await ordinaryCreate.text()).toBe(201)
  const ordinary = (await ordinaryCreate.json()) as { id: number; role: string }
  expect(ordinary.role).toBe('admin')

  const invalidRole = await request.post('/api/admin/admins', {
    data: { username: unique('invalid-role'), password: 'invalid-pass-12', role: 'owner' },
  })
  expect(invalidRole.status()).toBe(400)

  const missingTarget = await request.put('/api/admin/admins/999999999/role', {
    data: { role: 'admin' },
  })
  expect(missingTarget.status()).toBe(404)

  const selfRole = await request.put(`/api/admin/admins/${me.id}/role`, {
    data: { role: 'admin' },
  })
  expect(selfRole.status()).toBe(403)
  expect((await request.get('/api/admin/me')).ok()).toBeTruthy()
  const selfDelete = await request.delete(`/api/admin/admins/${me.id}`)
  expect(selfDelete.status()).toBe(403)
  expect((await request.get('/api/admin/me')).ok()).toBeTruthy()

  const extraSuperUsername = unique('extra-super')
  const extraSuperCreate = await request.post('/api/admin/admins', {
    data: {
      username: extraSuperUsername,
      password: 'extra-super-pass-12',
      role: 'super_admin',
    },
  })
  expect(extraSuperCreate.status(), await extraSuperCreate.text()).toBe(201)
  const extraSuper = (await extraSuperCreate.json()) as { id: number; role: string }
  expect(extraSuper.role).toBe('super_admin')

  const ordinaryRequest = await playwright.request.newContext({
    baseURL: API,
    extraHTTPHeaders: { Origin: WEB },
    storageState: { cookies: [], origins: [] },
  })
  const login = await ordinaryRequest.post('/api/admin/login', {
    data: { username: ordinaryUsername, password: ordinaryPassword },
  })
  expect(login.ok(), await login.text()).toBeTruthy()
  expect((await login.json()).role).toBe('admin')
  expect((await request.get('/api/admin/me')).ok()).toBeTruthy()

  const person = await ordinaryRequest.post('/api/admin/persons', {
    multipart: { name: unique('普通管理员神人') },
  })
  expect(person.status(), await person.text()).toBe(201)
  const personId = ((await person.json()) as { id: number }).id
  const quote = await ordinaryRequest.post('/api/admin/quotes', {
    data: { person_id: personId, content: unique('普通管理员言论') },
  })
  expect(quote.status(), await quote.text()).toBe(201)
  const quoteId = ((await quote.json()) as { id: string }).id
  expect((await ordinaryRequest.get('/api/admin/persons')).ok()).toBeTruthy()
  expect((await ordinaryRequest.get('/api/admin/quotes')).ok()).toBeTruthy()
  expect((await request.get('/api/admin/me')).ok()).toBeTruthy()

  const restrictedRequests: Array<{
    method: 'GET' | 'POST' | 'PUT' | 'DELETE'
    url: string
    data?: Record<string, unknown>
  }> = [
    { method: 'GET', url: '/api/admin/api-keys' },
    { method: 'POST', url: '/api/admin/api-keys', data: {} },
    { method: 'PUT', url: '/api/admin/api-keys/999999999', data: { name: 'forbidden-key' } },
    { method: 'DELETE', url: '/api/admin/api-keys/999999999' },
    { method: 'POST', url: '/api/admin/api-keys/999999999/reset-usage', data: {} },
    { method: 'GET', url: '/api/admin/settings' },
    {
      method: 'PUT',
      url: '/api/admin/settings',
      data: { site_name: 'forbidden', allow_propose_person: false },
    },
    { method: 'GET', url: '/api/admin/captcha' },
    { method: 'PUT', url: '/api/admin/captcha', data: { providers: [] } },
    { method: 'GET', url: '/api/admin/admins' },
    {
      method: 'POST',
      url: '/api/admin/admins',
      data: {},
    },
    { method: 'PUT', url: `/api/admin/admins/${me.id}/role`, data: { role: 'admin' } },
    { method: 'DELETE', url: `/api/admin/admins/${me.id}` },
  ]
  for (const entry of restrictedRequests) {
    const response = await ordinaryRequest.fetch(entry.url, {
      method: entry.method,
      data: entry.data,
    })
    expect(response.status(), `${entry.method} ${entry.url}: ${await response.text()}`).toBe(403)
  }
  expect((await request.get('/api/admin/me')).ok()).toBeTruthy()

  const promote = await request.put(`/api/admin/admins/${ordinary.id}/role`, {
    data: { role: 'super_admin' },
  })
  expect(promote.ok(), await promote.text()).toBeTruthy()
  expect((await ordinaryRequest.get('/api/admin/api-keys')).ok()).toBeTruthy()

  const demote = await request.put(`/api/admin/admins/${ordinary.id}/role`, {
    data: { role: 'admin' },
  })
  expect(demote.ok(), await demote.text()).toBeTruthy()
  expect((await ordinaryRequest.get('/api/admin/api-keys')).status()).toBe(403)

  expect((await ordinaryRequest.delete(`/api/admin/quotes/${quoteId}`)).ok()).toBeTruthy()
  expect((await ordinaryRequest.delete(`/api/admin/persons/${personId}`)).status()).toBe(204)
  await ordinaryRequest.dispose()

  expect((await request.delete(`/api/admin/admins/${ordinary.id}`)).status()).toBe(204)
  expect((await request.delete(`/api/admin/admins/${extraSuper.id}`)).status()).toBe(204)

  const loginAgain = await playwright.request.newContext({
    baseURL: API,
    extraHTTPHeaders: { Origin: WEB },
    storageState: { cookies: [], origins: [] },
  })
  const originalLogin = await loginAgain.post('/api/admin/login', {
    data: { username: ADMIN_USER, password: ADMIN_PASS },
  })
  expect(originalLogin.ok(), await originalLogin.text()).toBeTruthy()
  expect((await originalLogin.json()).role).toBe('super_admin')
  await loginAgain.dispose()
})
