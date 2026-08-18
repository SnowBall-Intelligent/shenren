import type { ApiErrorBody } from './types'

export class ApiError extends Error {
  status: number
  body: ApiErrorBody | null

  constructor(status: number, message: string, body: ApiErrorBody | null = null) {
    super(message)
    this.name = 'ApiError'
    this.status = status
    this.body = body
  }
}

async function parseError(res: Response): Promise<ApiError> {
  let body: ApiErrorBody | null = null
  try {
    body = (await res.json()) as ApiErrorBody
  } catch {
    /* ignore */
  }
  const message = body?.error || body?.message || res.statusText || `HTTP ${res.status}`
  return new ApiError(res.status, message, body)
}

type JsonOptions = Omit<RequestInit, 'body'> & { body?: unknown }

export async function apiJson<T>(path: string, options: JsonOptions = {}): Promise<T> {
  const { body, headers, ...rest } = options
  const res = await fetch(path, {
    credentials: 'include',
    headers: {
      ...(body !== undefined ? { 'Content-Type': 'application/json' } : {}),
      ...headers,
    },
    body: body !== undefined ? JSON.stringify(body) : undefined,
    ...rest,
  })
  if (!res.ok) {
    throw await parseError(res)
  }
  if (res.status === 204) {
    return undefined as T
  }
  const text = await res.text()
  if (!text) {
    return undefined as T
  }
  return JSON.parse(text) as T
}

export async function apiForm<T>(path: string, form: FormData, method: string = 'POST'): Promise<T> {
  const res = await fetch(path, {
    method,
    credentials: 'include',
    body: form,
  })
  if (!res.ok) {
    throw await parseError(res)
  }
  if (res.status === 204) {
    return undefined as T
  }
  const text = await res.text()
  if (!text) {
    return undefined as T
  }
  return JSON.parse(text) as T
}

/** Resolve avatar/upload paths for <img src>. */
export function uploadUrl(path: string | null | undefined): string | undefined {
  if (!path) return undefined
  if (path.startsWith('http://') || path.startsWith('https://') || path.startsWith('/')) {
    return path
  }
  return `/uploads/${path.replace(/^\.?\/?uploads\//, '')}`
}

/** First visible character for letter avatars (Unicode-aware). */
export function nameInitial(name: string | null | undefined): string {
  const trimmed = name?.trim() ?? ''
  if (!trimmed) return '?'
  return [...trimmed][0] ?? '?'
}
