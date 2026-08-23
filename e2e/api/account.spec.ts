import { readFile, readdir } from 'node:fs/promises'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { expect, test, type APIRequestContext } from '@playwright/test'
import { ensureAdmin } from '../helpers/api'
import { ADMIN_USER, API, unique, WEB } from '../helpers/env'

const root = path.join(path.dirname(fileURLToPath(import.meta.url)), '../..')
const auditDir = path.join(root, 'e2e/.tmp/runtime/data/logs/admin')

async function auditContents() {
  const names = (await readdir(auditDir)).filter(
    (name) => name.startsWith('audit-') && name.endsWith('.log'),
  )
  const files = await Promise.all(names.map((name) => readFile(path.join(auditDir, name), 'utf8')))
  return files.join('\n')
}

test('super and ordinary admins can securely update their own account', async ({
  playwright,
  request,
}) => {
  await ensureAdmin(request)
  const captchaResponse = await request.get('/api/admin/captcha')
  expect(captchaResponse.ok(), await captchaResponse.text()).toBeTruthy()
  const originalCaptcha = (await captchaResponse.json()) as {
    providers: Array<{ provider: string; site_key: string | null; secret: string | null }>
    account_update_enabled: boolean
  }

  const createdIds: number[] = []
  const contexts: APIRequestContext[] = []
  let ordinaryContext: APIRequestContext | null = null
  let ordinaryId = 0
  let ordinaryUsername = ''
  let ordinaryPassword = ''

  const createAndUpdate = async (role: 'super_admin' | 'admin') => {
    const initialUsername = unique(`${role}-self`)
    const initialPassword = `${role}-old-pass-12`
    const created = await request.post('/api/admin/admins', {
      data: { username: initialUsername, password: initialPassword, role },
    })
    expect(created.status(), await created.text()).toBe(201)
    const admin = (await created.json()) as { id: number }
    createdIds.push(admin.id)

    const context = await playwright.request.newContext({
      baseURL: API,
      extraHTTPHeaders: { Origin: WEB },
      storageState: { cookies: [], origins: [] },
    })
    contexts.push(context)
    const login = await context.post('/api/admin/login', {
      data: { username: initialUsername, password: initialPassword },
    })
    expect(login.ok(), await login.text()).toBeTruthy()

    const me = await context.get('/api/admin/me')
    expect(me.ok(), await me.text()).toBeTruthy()
    expect((await me.json()).captcha).toBeUndefined()

    const noChanges = await context.put('/api/admin/me', {
      data: { username: initialUsername, current_password: initialPassword },
    })
    expect(noChanges.status()).toBe(400)

    const wrongPassword = await context.put('/api/admin/me', {
      data: {
        username: `${initialUsername}-changed`,
        current_password: 'wrong-current-password',
      },
    })
    expect(wrongPassword.status()).toBe(400)

    const duplicate = await context.put('/api/admin/me', {
      data: { username: ADMIN_USER, current_password: initialPassword },
    })
    expect(duplicate.status()).toBe(409)

    const nextUsername = unique(`${role}-updated`)
    const nextPassword = `${role}-new-pass-34`
    const updated = await context.put('/api/admin/me', {
      data: {
        username: nextUsername,
        current_password: initialPassword,
        new_password: nextPassword,
      },
    })
    expect(updated.ok(), await updated.text()).toBeTruthy()
    const updatedBody = (await updated.json()) as { id: number; username: string; role: string }
    expect(updatedBody).toMatchObject({ id: admin.id, username: nextUsername, role })
    expect((await context.get('/api/admin/me')).ok()).toBeTruthy()

    const logout = await context.post('/api/admin/logout')
    expect(logout.status()).toBe(204)
    expect(
      (
        await context.post('/api/admin/login', {
          data: { username: initialUsername, password: initialPassword },
        })
      ).status(),
    ).toBe(401)
    const relogin = await context.post('/api/admin/login', {
      data: { username: nextUsername, password: nextPassword },
    })
    expect(relogin.ok(), await relogin.text()).toBeTruthy()

    return { context, id: admin.id, username: nextUsername, password: nextPassword }
  }

  try {
    const ordinary = await createAndUpdate('admin')
    ordinaryContext = ordinary.context
    ordinaryId = ordinary.id
    ordinaryUsername = ordinary.username
    ordinaryPassword = ordinary.password
    await createAndUpdate('super_admin')

    const invalidConfig = await request.put('/api/admin/captcha', {
      data: { providers: [], account_update_enabled: true },
    })
    expect(invalidConfig.status()).toBe(400)

    const enabled = await request.put('/api/admin/captcha', {
      data: {
        providers: [
          {
            provider: 'turnstile',
            site_key: '1x00000000000000000000AA',
            secret: '1x0000000000000000000000000000000AA',
          },
        ],
        account_update_enabled: true,
      },
    })
    expect(enabled.ok(), await enabled.text()).toBeTruthy()

    const protectedMe = await ordinaryContext.get('/api/admin/me')
    expect(protectedMe.ok(), await protectedMe.text()).toBeTruthy()
    const protectedBody = (await protectedMe.json()) as {
      captcha?: { providers?: Array<{ provider: string; site_key: string }> }
    }
    expect(protectedBody.captcha?.providers).toEqual([
      { provider: 'turnstile', site_key: '1x00000000000000000000AA' },
    ])

    const missingCaptcha = await ordinaryContext.put('/api/admin/me', {
      data: {
        username: ordinaryUsername,
        current_password: ordinaryPassword,
        new_password: 'captcha-blocked-pass-56',
      },
    })
    expect(missingCaptcha.status()).toBe(400)
    expect(await missingCaptcha.text()).toContain('人机验证')

    await expect
      .poll(async () => auditContents())
      .toMatch(
        new RegExp(
          `action="update_account" resource="admins" resource_id="?${ordinaryId}"?[^\n]*status=200`,
        ),
      )
    await expect
      .poll(async () => auditContents())
      .toMatch(/action="update_account" resource="admins"[^\n]*status=400/)
    expect(await auditContents()).not.toContain(ordinaryPassword)
  } finally {
    await request.put('/api/admin/captcha', {
      data: {
        providers: originalCaptcha.providers,
        account_update_enabled: originalCaptcha.account_update_enabled,
      },
    })
    for (const context of contexts) await context.dispose()
    for (const id of createdIds) await request.delete(`/api/admin/admins/${id}`)
  }
})
