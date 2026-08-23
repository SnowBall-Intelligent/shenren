import { expect, test } from '@playwright/test'
import { ADMIN_PASS, ADMIN_USER, unique } from '../helpers/env'

test('admin login reaches the review page', async ({ page }) => {
  await page.goto('/admin/login')
  await expect(page.getByRole('heading', { name: '管理员登录' })).toBeVisible()
  await page.getByLabel('用户名').fill(ADMIN_USER)
  await page.getByLabel('密码').fill(ADMIN_PASS)
  await page.getByRole('button', { name: '登录' }).click()
  await expect(page).toHaveURL(/\/admin\/quotes\/review/)
  await expect(page.getByRole('heading', { name: '言论审核' })).toBeVisible()
})

test('admin can create a person from the UI', async ({ page }) => {
  const name = unique('界面神人')
  await page.goto('/admin/login')
  await page.getByLabel('用户名').fill(ADMIN_USER)
  await page.getByLabel('密码').fill(ADMIN_PASS)
  await page.getByRole('button', { name: '登录' }).click()
  await expect(page).toHaveURL(/\/admin\/quotes/)

  await page.goto('/admin/persons')
  await page.getByRole('button', { name: '新增神人' }).click()
  await expect(page.getByRole('dialog')).toBeVisible()
  await page.getByLabel('名称').fill(name)
  await page.getByLabel('QQ 号获取头像（可选）').fill('123456789')
  await page.getByRole('button', { name: '保存' }).click()
  await expect(page.getByText(name)).toBeVisible()
})

test('theme preference persists across reloads', async ({ page }) => {
  await page.goto('/')
  await page.getByRole('button', { name: '外观模式' }).click()
  await page.getByRole('menuitem', { name: '深色' }).click()
  await expect.poll(() => page.evaluate(() => document.documentElement.style.colorScheme)).toBe('dark')
  await page.reload()
  await expect.poll(() => page.evaluate(() => document.documentElement.style.colorScheme)).toBe('dark')
})
