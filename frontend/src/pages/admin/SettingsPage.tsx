import { useEffect, useState } from 'react'
import {
  Alert,
  Box,
  Button,
  CircularProgress,
  FormControlLabel,
  Stack,
  Switch,
  TextField,
} from '@mui/material'
import { adminApi } from '../../api'
import type { SiteInfo } from '../../api/types'
import { ApiError } from '../../api/client'
import { useToast } from '../../components/AppToast'

export default function SettingsPage() {
  const toast = useToast()
  const [form, setForm] = useState<SiteInfo | null>(null)
  const [loading, setLoading] = useState(true)
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    adminApi
      .getSettings()
      .then(setForm)
      .catch((e) => setError(e instanceof ApiError ? e.message : '加载失败'))
      .finally(() => setLoading(false))
  }, [])

  const save = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!form) return
    setSaving(true)
    try {
      const updated = await adminApi.updateSettings({
        site_name: form.site_name,
        description: form.description,
        logo_url: form.logo_url,
        footer: form.footer,
        allow_propose_person: form.allow_propose_person,
      })
      setForm(updated)
      toast.fromSuccess(updated)
    } catch (err) {
      toast.fromError(err)
    } finally {
      setSaving(false)
    }
  }

  if (loading) {
    return (
      <Box sx={{ display: 'flex', justifyContent: 'center', py: 6 }}>
        <CircularProgress />
      </Box>
    )
  }

  if (!form) {
    return <Alert severity="error">{error ?? '无法加载设置'}</Alert>
  }

  return (
    <Box component="form" onSubmit={(e) => void save(e)} sx={{ maxWidth: 560 }}>
      <Stack spacing={2.5}>
        <TextField
          label="站点名称"
          required
          value={form.site_name}
          onChange={(e) => setForm({ ...form, site_name: e.target.value })}
        />
        <TextField
          label="简介"
          multiline
          minRows={2}
          value={form.description ?? ''}
          onChange={(e) => setForm({ ...form, description: e.target.value || null })}
        />
        <TextField
          label="Logo URL"
          value={form.logo_url ?? ''}
          onChange={(e) => setForm({ ...form, logo_url: e.target.value || null })}
          helperText="可为 /uploads/... 或完整 URL"
        />
        <TextField
          label="页脚"
          value={form.footer ?? ''}
          onChange={(e) => setForm({ ...form, footer: e.target.value || null })}
        />
        <FormControlLabel
          control={
            <Switch
              checked={form.allow_propose_person}
              onChange={(e) => setForm({ ...form, allow_propose_person: e.target.checked })}
            />
          }
          label="允许投稿时提出新神人名称"
        />
        <Button type="submit" variant="contained" disabled={saving} sx={{ alignSelf: 'flex-start' }}>
          {saving ? '保存中…' : '保存设置'}
        </Button>
      </Stack>
    </Box>
  )
}
