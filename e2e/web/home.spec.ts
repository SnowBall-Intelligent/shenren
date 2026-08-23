import { expect, test } from '@playwright/test'
import { createApprovedQuote, createPerson, ensureAdmin } from '../helpers/api'
import { API, unique, WEB } from '../helpers/env'

test('homepage shows the site chrome', async ({ page }) => {
  await page.goto('/')
  await expect(page.getByRole('link', { name: '神人网' })).toBeVisible()
  await expect(page.getByRole('button', { name: '投稿', exact: true })).toBeVisible()
  await expect(page.getByRole('link', { name: '管理' })).toBeVisible()
})

test('投稿 opens the submit dialog', async ({ page }) => {
  await page.goto('/')
  await page.getByRole('button', { name: '投稿', exact: true }).click()
  await expect(page.getByRole('dialog')).toBeVisible()
  await expect(page.getByRole('heading', { name: '投稿' })).toBeVisible()
  await expect(page.getByRole('combobox', { name: '神人', exact: true })).toBeVisible()
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

test('homepage timeline shows local publication metadata and appends paginated markers', async ({
  page,
  playwright,
}) => {
  const request = await playwright.request.newContext({
    baseURL: API,
    extraHTTPHeaders: { Origin: WEB },
  })
  await ensureAdmin(request)
  const personId = await createPerson(request, unique('时间轴神人'))
  const publishedAt = '2099-08-23T01:02:00Z'
  const content = unique('时间轴定位内容')
  const featured = await request.post('/api/admin/quotes', {
    data: { person_id: personId, content, published_at: publishedAt },
  })
  expect(featured.status(), await featured.text()).toBe(201)
  for (let index = 0; index < 20; index += 1) {
    await createApprovedQuote(request, personId, unique(`分页时间轴-${index}`))
  }
  const publicList = await request.get('/api/quotes?page=1&page_size=20')
  const total = ((await publicList.json()) as { total: number }).total
  await request.dispose()

  await page.goto('/')
  const items = page.getByTestId(/^quote-item-/)
  const markers = page.getByTestId(/^timeline-marker-/)
  await expect(items).toHaveCount(Math.min(total, 20))
  await expect(markers).toHaveCount(Math.min(total, 20))
  await expect(markers.first()).toHaveAttribute('aria-current', 'step')

  const quoteItem = items.filter({ hasText: content })
  await expect(quoteItem).toHaveCount(1)
  const itemTestId = await quoteItem.getAttribute('data-testid')
  const sequence = Number(itemTestId?.replace('quote-item-', ''))
  const expectedTime = await page.evaluate((iso) => {
    const parts = Object.fromEntries(
      new Intl.DateTimeFormat('zh-CN', {
        year: 'numeric',
        month: '2-digit',
        day: '2-digit',
        hour: '2-digit',
        minute: '2-digit',
        hourCycle: 'h23',
      })
        .formatToParts(new Date(iso))
        .filter((part) => part.type !== 'literal')
        .map((part) => [part.type, part.value]),
    )
    return `${parts.year}-${parts.month}-${parts.day} ${parts.hour}:${parts.minute}`
  }, publishedAt)
  await expect(quoteItem.getByTestId('quote-meta')).toHaveText(`#${sequence} · ${expectedTime}`)

  const marker = page.getByTestId(`timeline-marker-${sequence}`)
  await marker.hover()
  await expect(page.getByRole('tooltip')).toContainText(content)
  await marker.click()
  await expect
    .poll(() => quoteItem.evaluate((element) => Math.abs(element.getBoundingClientRect().top)))
    .toBeLessThan(500)

  await items.last().scrollIntoViewIfNeeded()
  await expect.poll(() => markers.count()).toBeGreaterThan(20)
})

test('mobile timeline opens a summary before locating a quote', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 })
  await page.goto('/')

  const timeline = page.getByTestId('quote-timeline')
  await expect(timeline).toBeVisible()
  await expect(timeline).toHaveCSS('position', 'sticky')
  const marker = page.getByTestId('timeline-marker-1')
  await marker.click()
  const preview = page.getByTestId('timeline-mobile-preview')
  const locateButton = page.getByRole('button', { name: '定位', exact: true })
  await expect(locateButton).toBeVisible()
  await expect(preview).toContainText(/^#1 · /)
  await locateButton.click()
  await expect(preview).not.toBeVisible()

  const overflow = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth)
  expect(overflow).toBeLessThanOrEqual(0)
})
