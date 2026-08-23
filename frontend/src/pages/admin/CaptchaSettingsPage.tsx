import { useEffect, useState } from 'react'
import {
  Alert,
  Box,
  Button,
  CircularProgress,
  FormControl,
  FormControlLabel,
  IconButton,
  InputLabel,
  MenuItem,
  Paper,
  Select,
  Stack,
  Switch,
  TextField,
  Typography,
} from '@mui/material'
import AddIcon from '@mui/icons-material/Add'
import ArrowUpwardIcon from '@mui/icons-material/ArrowUpward'
import ArrowDownwardIcon from '@mui/icons-material/ArrowDownward'
import DeleteIcon from '@mui/icons-material/Delete'
import { adminApi } from '../../api'
import type { CaptchaProviderConfig, CaptchaSettings, CaptchaVendor } from '../../api/types'
import { ApiError } from '../../api/client'
import { useToast } from '../../components/AppToast'

const VENDORS: { value: CaptchaVendor; label: string }[] = [
  { value: 'turnstile', label: 'Cloudflare Turnstile' },
  { value: 'recaptcha', label: 'reCAPTCHA（recaptcha.net）' },
  { value: 'geetest', label: '极验 v4' },
]

function helperFor(provider: CaptchaVendor): { site: string; secret: string; hint: string } {
  switch (provider) {
    case 'turnstile':
      return {
        site: 'Site Key',
        secret: 'Secret Key',
        hint: 'Cloudflare Dashboard → Turnstile',
      }
    case 'recaptcha':
      return {
        site: 'Site Key',
        secret: 'Secret Key',
        hint: 'Google reCAPTCHA 后台，v2「我不是机器人」勾选框',
      }
    case 'geetest':
      return {
        site: 'captcha_id',
        secret: 'captcha_key',
        hint: '极验后台的 ID 与 Key',
      }
  }
}

function emptyRow(provider: CaptchaVendor): CaptchaProviderConfig {
  return { provider, site_key: '', secret: '' }
}

export default function CaptchaSettingsPage() {
  const toast = useToast()
  const [form, setForm] = useState<CaptchaSettings | null>(null)
  const [loading, setLoading] = useState(true)
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    adminApi
      .getCaptcha()
      .then((data) =>
        setForm({
          providers: data.providers ?? [],
          account_update_enabled: data.account_update_enabled ?? false,
        }),
      )
      .catch((e) => setError(e instanceof ApiError ? e.message : '加载失败'))
      .finally(() => setLoading(false))
  }, [])

  const save = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!form) return
    setSaving(true)
    try {
      const updated = await adminApi.updateCaptcha({
        providers: form.providers,
        account_update_enabled: form.account_update_enabled,
      })
      setForm({
        providers: updated.providers ?? [],
        account_update_enabled: updated.account_update_enabled ?? false,
      })
      toast.fromSuccess(updated)
    } catch (err) {
      toast.fromError(err)
    } finally {
      setSaving(false)
    }
  }

  const unused = VENDORS.filter((v) => !form?.providers.some((p) => p.provider === v.value))

  const addProvider = (provider: CaptchaVendor) => {
    setForm((current) =>
      current
        ? { ...current, providers: [...current.providers, emptyRow(provider)] }
        : current,
    )
  }

  const updateAt = (index: number, patch: Partial<CaptchaProviderConfig>) => {
    setForm((current) =>
      current
        ? {
            ...current,
            providers: current.providers.map((item, i) =>
              i === index ? { ...item, ...patch } : item,
            ),
          }
        : current,
    )
  }

  const move = (index: number, dir: -1 | 1) => {
    setForm((current) => {
      if (!current) return current
      const next = [...current.providers]
      const target = index + dir
      if (target < 0 || target >= next.length) return current
      ;[next[index], next[target]] = [next[target], next[index]]
      return { ...current, providers: next }
    })
  }

  const removeAt = (index: number) => {
    setForm((current) => {
      if (!current) return current
      const providers = current.providers.filter((_, i) => i !== index)
      return {
        ...current,
        providers,
        account_update_enabled:
          providers.length > 0 ? current.account_update_enabled : false,
      }
    })
  }

  if (loading) {
    return (
      <Box sx={{ display: 'flex', justifyContent: 'center', py: 6 }}>
        <CircularProgress />
      </Box>
    )
  }

  if (!form) {
    return <Alert severity="error">{error ?? '无法加载人机验证设置'}</Alert>
  }

  return (
    <Box component="form" onSubmit={(e) => void save(e)} sx={{ maxWidth: 640 }}>
      <Stack spacing={2.5}>
        <Typography variant="body2" color="text.secondary">
          验证厂商按列表从上到下为优先顺序：先出第一个，加载超时、控件失败或服务端校验失败会自动改用下一个。
        </Typography>

        <Stack spacing={0.5}>
          <FormControlLabel
            control={
              <Switch
                checked={form.account_update_enabled}
                disabled={form.providers.length === 0}
                onChange={(event) =>
                  setForm({ ...form, account_update_enabled: event.target.checked })
                }
              />
            }
            label="修改管理员账号时要求人机验证"
          />
          <Typography variant="caption" color="text.secondary">
            前台投稿在配置厂商后始终启用验证；账号修改可通过此开关独立控制。
          </Typography>
        </Stack>

        {form.providers.length === 0 ? (
          <Alert severity="info">未配置任何人机验证时，投稿不校验。</Alert>
        ) : null}

        {form.providers.map((item, index) => {
          const labels = helperFor(item.provider)
          const options = VENDORS.filter(
            (v) => v.value === item.provider || unused.some((u) => u.value === v.value),
          )
          return (
            <Paper key={`${item.provider}-${index}`} variant="outlined" sx={{ p: 2 }}>
              <Stack spacing={2}>
                <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                  <Typography variant="subtitle2" sx={{ flexGrow: 1 }}>
                    优先 {index + 1}
                  </Typography>
                  <IconButton
                    size="small"
                    aria-label="上移"
                    disabled={index === 0}
                    onClick={() => move(index, -1)}
                  >
                    <ArrowUpwardIcon fontSize="small" />
                  </IconButton>
                  <IconButton
                    size="small"
                    aria-label="下移"
                    disabled={index === form.providers.length - 1}
                    onClick={() => move(index, 1)}
                  >
                    <ArrowDownwardIcon fontSize="small" />
                  </IconButton>
                  <IconButton size="small" aria-label="删除" onClick={() => removeAt(index)}>
                    <DeleteIcon fontSize="small" />
                  </IconButton>
                </Box>
                <FormControl fullWidth>
                  <InputLabel id={`captcha-provider-${index}`}>验证厂商</InputLabel>
                  <Select
                    labelId={`captcha-provider-${index}`}
                    label="验证厂商"
                    value={item.provider}
                    onChange={(e) =>
                      updateAt(index, { provider: e.target.value as CaptchaVendor })
                    }
                  >
                    {options.map((p) => (
                      <MenuItem key={p.value} value={p.value}>
                        {p.label}
                      </MenuItem>
                    ))}
                  </Select>
                </FormControl>
                <TextField
                  label={labels.site}
                  value={item.site_key ?? ''}
                  onChange={(e) => updateAt(index, { site_key: e.target.value })}
                  required
                />
                <TextField
                  label={labels.secret}
                  type="password"
                  autoComplete="off"
                  value={item.secret ?? ''}
                  onChange={(e) => updateAt(index, { secret: e.target.value })}
                  required
                />
                <Typography variant="caption" color="text.secondary">
                  {labels.hint}
                </Typography>
              </Stack>
            </Paper>
          )
        })}

        {unused.length > 0 ? (
          <Stack direction="row" spacing={1} sx={{ flexWrap: 'wrap' }}>
            {unused.map((p) => (
              <Button
                key={p.value}
                variant="outlined"
                size="small"
                startIcon={<AddIcon />}
                onClick={() => addProvider(p.value)}
              >
                {p.label}
              </Button>
            ))}
          </Stack>
        ) : null}

        <Button type="submit" variant="contained" disabled={saving} sx={{ alignSelf: 'flex-start' }}>
          {saving ? '保存中…' : '保存'}
        </Button>
      </Stack>
    </Box>
  )
}
