import { readFile, readdir } from 'node:fs/promises'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { expect, test } from '@playwright/test'
import { createPerson } from '../helpers/api'
import { unique } from '../helpers/env'

const root = path.join(path.dirname(fileURLToPath(import.meta.url)), '../..')
const logDir = path.join(root, 'e2e/.tmp/runtime/data/logs')
const auditDir = path.join(logDir, 'admin')

async function logContents(directory: string, prefix: string) {
  const names = (await readdir(directory)).filter(
    (name) => name.startsWith(`${prefix}-`) && name.endsWith('.log'),
  )
  const files = await Promise.all(names.map((name) => readFile(path.join(directory, name), 'utf8')))
  return files.join('\n')
}

const auditContents = () => logContents(auditDir, 'audit')
const systemContents = () => logContents(logDir, 'system')

test('admin writes produce success and failure audit records without leaking API keys', async ({ request }) => {
  const personId = await createPerson(request, unique('审计神人'))
  const removed = await request.delete(`/api/admin/persons/${personId}`)
  expect(removed.status(), await removed.text()).toBe(204)
  const missing = await request.delete(`/api/admin/persons/${personId}`)
  expect(missing.status(), await missing.text()).toBe(404)

  const createdKey = await request.post('/api/admin/api-keys', {
    data: { name: unique('审计Key') },
  })
  expect(createdKey.status(), await createdKey.text()).toBe(201)
  const createdKeyBody = (await createdKey.json()) as { id: number; key: string }
  const rawKey = createdKeyBody.key

  await expect
    .poll(async () => auditContents())
    .toMatch(new RegExp(`action="delete" resource="persons" resource_id="?${personId}"?[^\n]*status=204`))
  await expect
    .poll(async () => auditContents())
    .toMatch(new RegExp(`action="delete" resource="persons" resource_id="?${personId}"?[^\n]*status=404`))
  await expect
    .poll(async () => auditContents())
    .toMatch(
      new RegExp(
        `action="create" resource="api_keys" resource_id="?${createdKeyBody.id}"?[^\n]*status=201`,
      ),
    )
  const allLogs = `${await systemContents()}\n${await auditContents()}`
  expect(allLogs).not.toContain(rawKey)
  expect(await systemContents()).not.toContain('shenren::audit')
})
