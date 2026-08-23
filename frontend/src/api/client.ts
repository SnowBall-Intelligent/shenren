import type { ApiErrorBody } from './types'

/** API origin without trailing slash. Empty = same-origin (Vite `/api` proxy in dev). */
export function apiBase(): string {
  const raw = import.meta.env.VITE_API_URL
  if (typeof raw !== 'string') return ''
  return raw.trim().replace(/\/+$/, '')
}

/** Resolve `/api/...` or `/uploads/...` against `VITE_API_URL`. */
export function apiUrl(path: string): string {
  if (path.startsWith('http://') || path.startsWith('https://')) return path
  const base = apiBase()
  const p = path.startsWith('/') ? path : `/${path}`
  return base ? `${base}${p}` : p
}

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
  const message = body?.message || body?.error || res.statusText || `HTTP ${res.status}`
  return new ApiError(res.status, message, body)
}

type JsonOptions = Omit<RequestInit, 'body'> & { body?: unknown }

export async function apiJson<T>(path: string, options: JsonOptions = {}): Promise<T> {
  const { body, headers, ...rest } = options
  const res = await fetch(apiUrl(path), {
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
  const res = await fetch(apiUrl(path), {
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
  if (path.startsWith('http://') || path.startsWith('https://')) return path
  if (path.startsWith('/')) return apiUrl(path)
  return apiUrl(`/uploads/${path.replace(/^\.?\/?uploads\//, '')}`)
}

/** First visible character for letter avatars (Unicode-aware). */
export function nameInitial(name: string | null | undefined): string {
  const trimmed = name?.trim() ?? ''
  if (!trimmed) return '?'
  return [...trimmed][0] ?? '?'
}

export function qqAvatarUrl(qq: string): string | undefined {
  const value = qq.trim()
  return /^[1-9]\d{4,19}$/.test(value)
    ? `https://q2.qlogo.cn/headimg_dl?dst_uin=${value}&spec=0`
    : undefined
}
