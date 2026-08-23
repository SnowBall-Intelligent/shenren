/** Shared API types — snake_case to match Axum/Serde defaults. */

export type CaptchaVendor = 'turnstile' | 'recaptcha' | 'geetest'
export type CaptchaProvider = 'none' | CaptchaVendor

export interface PublicCaptchaProvider {
  provider: CaptchaVendor
  site_key: string
}

export interface PublicCaptcha {
  providers?: PublicCaptchaProvider[]
  provider?: CaptchaProvider
  site_key?: string
}

export interface CaptchaPayload {
  provider?: CaptchaVendor
  token?: string
  lot_number?: string
  captcha_output?: string
  pass_token?: string
  gen_time?: string
}

export interface CaptchaProviderConfig {
  provider: CaptchaVendor
  site_key: string | null
  secret: string | null
}

export interface CaptchaSettings {
  providers: CaptchaProviderConfig[]
  message?: string
}

export interface SiteInfo {
  site_name: string
  description: string | null
  logo_url: string | null
  footer: string | null
  allow_propose_person: boolean
  captcha?: PublicCaptcha
  message?: string
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
  id: string
  person_id?: number | null
  proposed_person_name?: string | null
  proposed_person_avatar_url?: string | null
  content: string
  source: string | null
  status?: 'pending' | 'approved' | 'rejected'
  pinned?: boolean
  published_at?: string
  created_at: string
  reviewed_at?: string | null
  reviewed_by?: number | null
  person: QuotePerson | null
  message?: string
}

export interface QuoteWrite {
  person_id: number
  content: string
  source?: string | null
  pinned?: boolean
  published_at?: string | null
  place_before_id?: string | null
  place_after_id?: string | null
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
  proposed_person_qq?: string | null
  content: string
  source?: string | null
  published_at?: string | null
  place_before_id?: string | null
  place_after_id?: string | null
  captcha?: CaptchaPayload
}

export interface BootstrapStatus {
  needs_setup: boolean
  has_admins?: boolean
}

export type AdminRole = 'super_admin' | 'admin'

export interface Admin {
  id: number
  username: string
  role: AdminRole
  created_at: string
}

export interface AdminMe {
  id: number
  username: string
  role: AdminRole
  message?: string
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
  captcha_fallback?: boolean
}

export interface ApiKey {
  id: number
  name: string
  key_prefix: string
  enabled: boolean
  rate_limit: number | null
  rate_window_secs: number | null
  total_quota: number | null
  used_count: number
  concurrency_limit: number | null
  allowed_ips: string[]
  allowed_domains: string[]
  created_at: string
  updated_at: string
  last_used_at: string | null
  key?: string
}

export interface ApiKeyWrite {
  name: string
  enabled: boolean
  rate_limit: number | null
  rate_window_secs: number | null
  total_quota: number | null
  concurrency_limit: number | null
  allowed_ips: string[]
  allowed_domains: string[]
}
