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
  message?: string
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
  captcha?: CaptchaPayload
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
