import { type APIRequestContext, expect } from '@playwright/test'
import { ADMIN_PASS, ADMIN_USER } from './env'

export async function ensureAdmin(request: APIRequestContext): Promise<void> {
  const me = await request.get('/api/admin/me')
  if (me.ok()) return

  const statusRes = await request.get('/api/admin/bootstrap-status')
  expect(statusRes.ok()).toBeTruthy()
  const status = (await statusRes.json()) as { needs_setup: boolean }
  if (status.needs_setup) {
    const setup = await request.post('/api/admin/setup', {
      data: { username: ADMIN_USER, password: ADMIN_PASS },
    })
    expect(setup.status(), await setup.text()).toBe(201)
    return
  }
  const login = await request.post('/api/admin/login', {
    data: { username: ADMIN_USER, password: ADMIN_PASS },
  })
  expect(login.ok(), await login.text()).toBeTruthy()
}

export async function createPerson(request: APIRequestContext, name: string): Promise<number> {
  const res = await request.post('/api/admin/persons', {
    multipart: { name },
  })
  expect(res.status(), await res.text()).toBe(201)
  const body = (await res.json()) as { id: number }
  return body.id
}

export async function createApprovedQuote(
  request: APIRequestContext,
  personId: number,
  content: string,
): Promise<string> {
  const res = await request.post('/api/admin/quotes', {
    data: { person_id: personId, content },
  })
  expect(res.status(), await res.text()).toBe(201)
  const body = (await res.json()) as { id: string }
  return body.id
}
