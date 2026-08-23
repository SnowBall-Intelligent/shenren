import { apiForm, apiJson } from './client'
import type {
  Admin,
  AdminMe,
  BootstrapStatus,
  Paginated,
  Person,
  CaptchaSettings,
  Quote,
  QuoteWrite,
  SiteInfo,
  SiteSettingsUpdate,
  SubmissionPayload,
  ApiKey,
  ApiKeyWrite,
} from './types'

export const publicApi = {
  getSite: () => apiJson<SiteInfo>('/api/site'),

  getQuotes: (
    page = 1,
    pageSize = 20,
    personId?: number,
    extras?: { q?: string; pinned?: boolean; recent?: boolean },
  ) => {
    const q = new URLSearchParams()
    q.set('page', String(page))
    q.set('page_size', String(pageSize))
    if (personId != null) q.set('person_id', String(personId))
    if (extras?.q?.trim()) q.set('q', extras.q.trim())
    if (extras?.pinned != null) q.set('pinned', String(extras.pinned))
    if (extras?.recent) q.set('recent', 'true')
    return apiJson<Paginated<Quote>>(`/api/quotes?${q}`)
  },

  getPersons: (q?: string, limit = 50) => {
    const params = new URLSearchParams()
    if (q) params.set('q', q)
    params.set('limit', String(limit))
    return apiJson<Person[] | { items: Person[] }>(`/api/persons?${params}`)
  },

  submit: (payload: SubmissionPayload) =>
    apiJson<{ id: string; status?: string; message?: string }>('/api/submissions', {
      method: 'POST',
      body: payload,
    }),
}

export function normalizePersons(data: Person[] | { items: Person[] }): Person[] {
  return Array.isArray(data) ? data : data.items
}

export const adminApi = {
  bootstrapStatus: () => apiJson<BootstrapStatus>('/api/admin/bootstrap-status'),

  setup: (username: string, password: string) =>
    apiJson<AdminMe>('/api/admin/setup', {
      method: 'POST',
      body: { username, password },
    }),

  login: (username: string, password: string) =>
    apiJson<AdminMe>('/api/admin/login', {
      method: 'POST',
      body: { username, password },
    }),

  logout: () => apiJson<void>('/api/admin/logout', { method: 'POST' }),

  me: () => apiJson<AdminMe>('/api/admin/me'),

  // Quotes review
  listQuotes: (
    params: {
      status?: string
      page?: number
      page_size?: number
      q?: string
      pinned?: boolean
      recent?: boolean
    } = {},
  ) => {
    const q = new URLSearchParams()
    if (params.status) q.set('status', params.status)
    q.set('page', String(params.page ?? 1))
    q.set('page_size', String(params.page_size ?? 20))
    if (params.q?.trim()) q.set('q', params.q.trim())
    if (params.pinned != null) q.set('pinned', String(params.pinned))
    if (params.recent) q.set('recent', 'true')
    return apiJson<Paginated<Quote>>(`/api/admin/quotes?${q}`)
  },

  approveQuote: (
    id: string,
    body?: {
      person_id?: number
      create_person_name?: string
      qq?: string
      avatar_url?: string
    },
  ) =>
    apiJson<Quote>(`/api/admin/quotes/${id}/approve-json`, {
      method: 'POST',
      body: body ?? {},
    }),

  /** Approve while uploading a new person avatar (multipart). */
  approveQuoteWithAvatar: (id: string, form: FormData) =>
    apiForm<Quote>(`/api/admin/quotes/${id}/approve`, form),

  rejectQuote: (id: string) =>
    apiJson<Quote>(`/api/admin/quotes/${id}/reject`, { method: 'POST', body: {} }),

  createQuote: (body: QuoteWrite) =>
    apiJson<Quote & { message?: string }>('/api/admin/quotes', { method: 'POST', body }),

  updateQuote: (id: string, body: QuoteWrite) =>
    apiJson<Quote & { message?: string }>(`/api/admin/quotes/${id}`, { method: 'PUT', body }),

  moveQuote: (id: string, direction: 'up' | 'down') =>
    apiJson<Quote & { message?: string }>(`/api/admin/quotes/${id}/move`, {
      method: 'POST',
      body: { direction },
    }),

  reorderQuotes: (ids: string[]) =>
    apiJson<{ message?: string }>('/api/admin/quotes/reorder', {
      method: 'POST',
      body: { ids },
    }),

  deleteQuote: (id: string) =>
    apiJson<{ message?: string }>(`/api/admin/quotes/${id}`, { method: 'DELETE' }),

  // Persons
  listPersons: (params: { page?: number; page_size?: number } = {}) => {
    const q = new URLSearchParams()
    q.set('page', String(params.page ?? 1))
    q.set('page_size', String(params.page_size ?? 20))
    return apiJson<Paginated<Person>>(`/api/admin/persons?${q}`)
  },

  createPerson: (form: FormData) => apiForm<Person>('/api/admin/persons', form),

  updatePerson: (id: number, form: FormData) =>
    apiForm<Person>(`/api/admin/persons/${id}`, form, 'PUT'),

  deletePerson: (id: number) =>
    apiJson<void>(`/api/admin/persons/${id}`, { method: 'DELETE' }),

  // Settings
  getSettings: () => apiJson<SiteInfo>('/api/admin/settings'),

  updateSettings: (body: SiteSettingsUpdate) =>
    apiJson<SiteInfo>('/api/admin/settings', { method: 'PUT', body }),

  getCaptcha: () => apiJson<CaptchaSettings>('/api/admin/captcha'),

  updateCaptcha: (body: { providers: CaptchaSettings['providers'] }) =>
    apiJson<CaptchaSettings>('/api/admin/captcha', { method: 'PUT', body }),

  // Admins
  listAdmins: () => apiJson<Admin[] | { items: Admin[] }>('/api/admin/admins'),

  createAdmin: (username: string, password: string) =>
    apiJson<Admin>('/api/admin/admins', {
      method: 'POST',
      body: { username, password },
    }),

  deleteAdmin: (id: number) =>
    apiJson<void>(`/api/admin/admins/${id}`, { method: 'DELETE' }),

  listApiKeys: () => apiJson<ApiKey[]>('/api/admin/api-keys'),

  createApiKey: (body: ApiKeyWrite) =>
    apiJson<ApiKey>('/api/admin/api-keys', { method: 'POST', body }),

  updateApiKey: (id: number, body: ApiKeyWrite) =>
    apiJson<ApiKey>(`/api/admin/api-keys/${id}`, { method: 'PUT', body }),

  resetApiKeyUsage: (id: number) =>
    apiJson<ApiKey>(`/api/admin/api-keys/${id}/reset-usage`, { method: 'POST', body: {} }),

  deleteApiKey: (id: number) =>
    apiJson<{ message: string }>(`/api/admin/api-keys/${id}`, { method: 'DELETE' }),
}

export function normalizeAdmins(data: Admin[] | { items: Admin[] }): Admin[] {
  return Array.isArray(data) ? data : data.items
}
