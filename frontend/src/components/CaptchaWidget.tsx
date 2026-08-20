import { useCallback, useEffect, useRef, useState } from 'react'
import { Alert, Box, CircularProgress, Typography } from '@mui/material'
import type { CaptchaPayload, CaptchaVendor, PublicCaptchaProvider } from '../api/types'

type Props = {
  providers: PublicCaptchaProvider[]
  onChange: (payload: CaptchaPayload | null) => void
  skipSignal?: number
  onExhausted?: (message: string) => void
}

type GeetestObj = {
  appendTo: (el: HTMLElement | string) => void
  getValidate: () => {
    lot_number?: string
    captcha_output?: string
    pass_token?: string
    gen_time?: string
  }
  onSuccess: (cb: () => void) => void
  onError?: (cb: () => void) => void
  onClose?: (cb: () => void) => void
  destroy?: () => void
}

declare global {
  interface Window {
    turnstile?: {
      render: (el: HTMLElement, opts: Record<string, unknown>) => string
      remove: (id: string) => void
    }
    grecaptcha?: {
      ready: (cb: () => void) => void
      render: (el: HTMLElement, opts: Record<string, unknown>) => number
      reset: (id?: number) => void
    }
    initGeetest4?: (
      config: Record<string, unknown>,
      callback: (captcha: GeetestObj) => void,
    ) => void
  }
}

const LABELS: Record<CaptchaVendor, string> = {
  turnstile: 'Cloudflare Turnstile',
  recaptcha: 'reCAPTCHA',
  geetest: '极验',
}

function loadScript(src: string, id: string): Promise<void> {
  return new Promise((resolve, reject) => {
    const existing = document.getElementById(id) as HTMLScriptElement | null
    if (existing) {
      if (existing.dataset.loaded === '1') {
        resolve()
        return
      }
      existing.addEventListener('load', () => resolve(), { once: true })
      existing.addEventListener('error', () => reject(new Error('脚本加载失败')), { once: true })
      return
    }
    const script = document.createElement('script')
    script.id = id
    script.src = src
    script.async = true
    script.onload = () => {
      script.dataset.loaded = '1'
      resolve()
    }
    script.onerror = () => reject(new Error('脚本加载失败'))
    document.head.appendChild(script)
  })
}

function waitFor<T>(getter: () => T | undefined, timeoutMs = 15000): Promise<T> {
  return new Promise((resolve, reject) => {
    const started = Date.now()
    const tick = () => {
      const value = getter()
      if (value) {
        resolve(value)
        return
      }
      if (Date.now() - started > timeoutMs) {
        reject(new Error('人机验证加载超时'))
        return
      }
      window.setTimeout(tick, 50)
    }
    tick()
  })
}

function SingleWidget({
  provider,
  siteKey,
  onChange,
  onFail,
}: {
  provider: CaptchaVendor
  siteKey: string
  onChange: (payload: CaptchaPayload | null) => void
  onFail: (reason: string) => void
}) {
  const hostRef = useRef<HTMLDivElement>(null)
  const onChangeRef = useRef(onChange)
  const onFailRef = useRef(onFail)
  onChangeRef.current = onChange
  onFailRef.current = onFail
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    const host = hostRef.current
    if (!host) {
      onFailRef.current('人机验证加载失败')
      return
    }

    let cancelled = false
    let turnstileId: string | null = null
    let recaptchaId: number | null = null
    let geetest: GeetestObj | null = null

    const emit = (payload: CaptchaPayload | null) => {
      if (!cancelled) onChangeRef.current(payload)
    }

    const fail = (message: string) => {
      if (cancelled) return
      onChangeRef.current(null)
      onFailRef.current(message)
    }

    setLoading(true)

    const run = async () => {
      try {
        if (provider === 'turnstile') {
          await loadScript(
            'https://challenges.cloudflare.com/turnstile/v0/api.js?render=explicit',
            'cf-turnstile-script',
          )
          const turnstile = await waitFor(() => window.turnstile)
          if (cancelled) return
          turnstileId = turnstile.render(host, {
            sitekey: siteKey,
            theme: 'dark',
            callback: (token: string) => emit({ provider, token }),
            'expired-callback': () => emit(null),
            'error-callback': () => fail('Turnstile 验证失败'),
          })
        } else if (provider === 'recaptcha') {
          await loadScript(
            'https://www.recaptcha.net/recaptcha/api.js?render=explicit',
            'recaptcha-net-script',
          )
          const grecaptcha = await waitFor(() => window.grecaptcha)
          await new Promise<void>((resolve) => grecaptcha.ready(() => resolve()))
          if (cancelled) return
          recaptchaId = grecaptcha.render(host, {
            sitekey: siteKey,
            theme: 'dark',
            callback: (token: string) => emit({ provider, token }),
            'expired-callback': () => emit(null),
            'error-callback': () => fail('reCAPTCHA 验证失败'),
          })
        } else if (provider === 'geetest') {
          await loadScript('https://static.geetest.com/v4/gt4.js', 'geetest-gt4-script')
          const initGeetest4 = await waitFor(() => window.initGeetest4)
          if (cancelled) return
          await new Promise<void>((resolve, reject) => {
            let settled = false
            const timer = window.setTimeout(() => {
              if (!settled) {
                settled = true
                reject(new Error('极验初始化超时'))
              }
            }, 15000)
            initGeetest4(
              {
                captchaId: siteKey,
                product: 'float',
                language: 'zho',
              },
              (captcha) => {
                if (settled || cancelled) {
                  captcha.destroy?.()
                  return
                }
                settled = true
                window.clearTimeout(timer)
                geetest = captcha
                captcha.appendTo(host)
                captcha.onSuccess(() => {
                  const result = captcha.getValidate()
                  if (
                    result?.lot_number &&
                    result.captcha_output &&
                    result.pass_token &&
                    result.gen_time
                  ) {
                    emit({
                      provider,
                      lot_number: result.lot_number,
                      captcha_output: result.captcha_output,
                      pass_token: result.pass_token,
                      gen_time: result.gen_time,
                    })
                  } else {
                    fail('极验验证失败')
                  }
                })
                captcha.onError?.(() => fail('极验验证失败'))
                captcha.onClose?.(() => emit(null))
                resolve()
              },
            )
          })
        }
        if (!cancelled) setLoading(false)
      } catch (e) {
        fail(e instanceof Error ? e.message : '人机验证加载失败')
      }
    }

    void run()

    return () => {
      cancelled = true
      if (turnstileId && window.turnstile) {
        try {
          window.turnstile.remove(turnstileId)
        } catch {
          /* ignore */
        }
      }
      if (recaptchaId != null && window.grecaptcha) {
        try {
          window.grecaptcha.reset(recaptchaId)
        } catch {
          /* ignore */
        }
        host.replaceChildren()
      }
      geetest?.destroy?.()
    }
  }, [provider, siteKey])

  return (
    <Box>
      {loading ? (
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, py: 1 }}>
          <CircularProgress size={18} />
          <Typography variant="body2" color="text.secondary">
            加载{LABELS[provider]}…
          </Typography>
        </Box>
      ) : null}
      <Box ref={hostRef} sx={{ minHeight: 72 }} />
    </Box>
  )
}

export function publicCaptchaList(captcha: {
  providers?: PublicCaptchaProvider[]
  provider?: string
  site_key?: string
} | undefined): PublicCaptchaProvider[] {
  if (captcha?.providers?.length) {
    return captcha.providers.filter((p) => Boolean(p.site_key))
  }
  if (
    captcha?.provider &&
    captcha.provider !== 'none' &&
    captcha.site_key
  ) {
    return [{ provider: captcha.provider as CaptchaVendor, site_key: captcha.site_key }]
  }
  return []
}

export default function CaptchaWidget({ providers, onChange, skipSignal = 0, onExhausted }: Props) {
  const [index, setIndex] = useState(0)
  const [exhausted, setExhausted] = useState(false)
  const [lastError, setLastError] = useState<string | null>(null)
  const skipSeen = useRef(0)
  const skipSignalRef = useRef(skipSignal)
  skipSignalRef.current = skipSignal
  const onChangeRef = useRef(onChange)
  onChangeRef.current = onChange
  const onExhaustedRef = useRef(onExhausted)
  onExhaustedRef.current = onExhausted
  const chainKey = providers.map((p) => `${p.provider}:${p.site_key}`).join('|')

  useEffect(() => {
    setIndex(0)
    setExhausted(false)
    setLastError(null)
    skipSeen.current = skipSignalRef.current
  }, [chainKey])

  const goNext = useCallback((reason: string) => {
    setIndex((current) => {
      const next = current + 1
      if (next < providers.length) {
        queueMicrotask(() => onChangeRef.current(null))
        return next
      }
      queueMicrotask(() => {
        onChangeRef.current(null)
        setExhausted(true)
        setLastError(reason)
        onExhaustedRef.current?.(reason)
      })
      return current
    })
  }, [providers.length])

  useEffect(() => {
    if (skipSignal <= skipSeen.current) return
    skipSeen.current = skipSignal
    goNext('人机验证未通过')
  }, [goNext, skipSignal])

  const handleFail = useCallback(
    (reason: string) => {
      goNext(reason)
    },
    [goNext],
  )

  if (!providers.length) return null

  const current = providers[Math.min(index, providers.length - 1)]
  if (!current) return null

  if (exhausted) {
    return <Alert severity="error">{lastError ?? '人机验证失败，请稍后重试'}</Alert>
  }

  return (
    <Box>
      {index > 0 ? (
        <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mb: 1 }}>
          上一验证不可用，已切换到{LABELS[current.provider]}（{index + 1}/{providers.length}）
        </Typography>
      ) : null}
      <SingleWidget
        key={`${current.provider}-${index}`}
        provider={current.provider}
        siteKey={current.site_key}
        onChange={onChange}
        onFail={handleFail}
      />
    </Box>
  )
}
