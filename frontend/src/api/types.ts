/** Shared API types — snake_case to match Axum/Serde defaults. */

export interface SiteInfo {
  site_name: string
  description: string | null
  logo_url: string | null
  footer: string | null
  allow_propose_person: boolean
}

export interface Person {
  id: number
  name: string
  avatar_url: string
  created_at?: string
}

export interface QuotePerson {
  id: number
  name: string
  avatar_url: string
}

export interface Quote {
  id: number
  person_id?: number | null
  proposed_person_name?: string | null
  content: string
  source: string | null
  status?: 'pending' | 'approved' | 'rejected'
  created_at: string
  reviewed_at?: string | null
  reviewed_by?: number | null
  person: QuotePerson | null
}

export interface Paginated<T> {
  items: T[]
  total: number
  page: number
  page_size: number
}

export interface SubmissionPayload {
  person_id?: number | null
  proposed_person_name?: string | null
  content: string
  source?: string | null
}

export interface BootstrapStatus {
  needs_setup: boolean
  has_admins?: boolean
}

export interface Admin {
  id: number
  username: string
  created_at: string
}

export interface AdminMe {
  id: number
  username: string
}

export interface SiteSettingsUpdate {
  site_name: string
  description?: string | null
  logo_url?: string | null
  footer?: string | null
  allow_propose_person: boolean
}

export interface ApiErrorBody {
  error?: string
  message?: string
}
