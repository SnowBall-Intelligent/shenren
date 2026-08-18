import { apiForm, apiJson } from './client'
import type {
  Admin,
  AdminMe,
  BootstrapStatus,
  Paginated,
  Person,
  Quote,
  SiteInfo,
  SiteSettingsUpdate,
  SubmissionPayload,
} from './types'

export const publicApi = {
  getSite: () => apiJson<SiteInfo>('/api/site'),

  getQuotes: (page = 1, pageSize = 20) =>
    apiJson<Paginated<Quote>>(`/api/quotes?page=${page}&page_size=${pageSize}`),

  getPersons: () => apiJson<Person[] | { items: Person[] }>('/api/persons'),

  submit: (payload: SubmissionPayload) =>
    apiJson<{ id: number }>('/api/submissions', { method: 'POST', body: payload }),
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
  listQuotes: (params: { status?: string; page?: number; page_size?: number } = {}) => {
    const q = new URLSearchParams()
    if (params.status) q.set('status', params.status)
    q.set('page', String(params.page ?? 1))
    q.set('page_size', String(params.page_size ?? 20))
    return apiJson<Paginated<Quote>>(`/api/admin/quotes?${q}`)
  },

  approveQuote: (
    id: number,
    body?: {
      person_id?: number
      create_person_name?: string
      avatar_url?: string
    },
  ) =>
    apiJson<Quote>(`/api/admin/quotes/${id}/approve-json`, {
      method: 'POST',
      body: body ?? {},
    }),

  /** Approve while uploading a new person avatar (multipart). */
  approveQuoteWithAvatar: (id: number, form: FormData) =>
    apiForm<Quote>(`/api/admin/quotes/${id}/approve`, form),

  rejectQuote: (id: number) =>
    apiJson<Quote>(`/api/admin/quotes/${id}/reject`, { method: 'POST', body: {} }),

  createQuote: (body: { person_id: number; content: string; source?: string | null }) =>
    apiJson<Quote>('/api/admin/quotes', { method: 'POST', body }),

  // Persons
  listPersons: () => apiJson<Person[] | { items: Person[] }>('/api/admin/persons'),

  createPerson: (form: FormData) => apiForm<Person>('/api/admin/persons', form),

  updatePerson: (id: number, form: FormData) =>
    apiForm<Person>(`/api/admin/persons/${id}`, form, 'PUT'),

  deletePerson: (id: number) =>
    apiJson<void>(`/api/admin/persons/${id}`, { method: 'DELETE' }),

  // Settings
  getSettings: () => apiJson<SiteInfo>('/api/admin/settings'),

  updateSettings: (body: SiteSettingsUpdate) =>
    apiJson<SiteInfo>('/api/admin/settings', { method: 'PUT', body }),

  // Admins
  listAdmins: () => apiJson<Admin[] | { items: Admin[] }>('/api/admin/admins'),

  createAdmin: (username: string, password: string) =>
    apiJson<Admin>('/api/admin/admins', {
      method: 'POST',
      body: { username, password },
    }),

  deleteAdmin: (id: number) =>
    apiJson<void>(`/api/admin/admins/${id}`, { method: 'DELETE' }),
}

export function normalizeAdmins(data: Admin[] | { items: Admin[] }): Admin[] {
  return Array.isArray(data) ? data : data.items
}
