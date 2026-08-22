import { expect, test } from '@playwright/test'
import { createApprovedQuote, createPerson, ensureAdmin } from '../helpers/api'
import { API, unique, WEB } from '../helpers/env'

test('homepage shows the site chrome', async ({ page }) => {
  await page.goto('/')
  await expect(page.getByRole('link', { name: '神人网' })).toBeVisible()
  await expect(page.getByRole('button', { name: '投稿' })).toBeVisible()
  await expect(page.getByRole('link', { name: '管理' })).toBeVisible()
})

test('投稿 opens the submit dialog', async ({ page }) => {
  await page.goto('/')
  await page.getByRole('button', { name: '投稿' }).click()
  await expect(page.getByRole('dialog')).toBeVisible()
  await expect(page.getByRole('heading', { name: '投稿' })).toBeVisible()
  await expect(page.getByLabel('神人')).toBeVisible()
})

test('an approved quote created via API appears on the homepage', async ({ page, playwright }) => {
  const request = await playwright.request.newContext({
    baseURL: API,
    extraHTTPHeaders: { Origin: WEB },
  })
  await ensureAdmin(request)
  const name = unique('首页神人')
  const content = unique('首页应当看见')
  const personId = await createPerson(request, name)
  await createApprovedQuote(request, personId, content)
  await request.dispose()

  await page.goto('/')
  await expect(page.getByText(name)).toBeVisible()
  await expect(page.getByText(content)).toBeVisible()
})
