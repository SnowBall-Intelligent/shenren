import { useCallback, useEffect, useState } from 'react'
import {
  Alert,
  Box,
  Button,
  Chip,
  CircularProgress,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  FormControlLabel,
  IconButton,
  Stack,
  Switch,
  TextField,
  Tooltip,
  Typography,
} from '@mui/material'
import AddIcon from '@mui/icons-material/Add'
import ContentCopyIcon from '@mui/icons-material/ContentCopy'
import DeleteIcon from '@mui/icons-material/Delete'
import EditIcon from '@mui/icons-material/Edit'
import RestartAltIcon from '@mui/icons-material/RestartAlt'
import { adminApi } from '../../api'
import type { ApiKey, ApiKeyWrite } from '../../api/types'
import { ApiError } from '../../api/client'
import { useToast } from '../../components/AppToast'

export default function ApiKeysPage() {
  const toast = useToast()
  const [items, setItems] = useState<ApiKey[]>([])
  const [loading, setLoading] = useState(true)
  const [editing, setEditing] = useState<'create' | ApiKey | null>(null)
  const [createdKey, setCreatedKey] = useState<string | null>(null)

  const load = useCallback(async () => {
    setLoading(true)
    try {
      setItems(await adminApi.listApiKeys())
    } catch (error) {
      toast.fromError(error)
    } finally {
      setLoading(false)
    }
  }, [toast])

  useEffect(() => {
    void load()
  }, [load])

  const resetUsage = async (item: ApiKey) => {
    if (!window.confirm(`确定重置“${item.name}”的已用额度？`)) return
    try {
      await adminApi.resetApiKeyUsage(item.id)
      await load()
    } catch (error) {
      toast.fromError(error)
    }
  }

  const remove = async (item: ApiKey) => {
    if (!window.confirm(`确定删除“${item.name}”？使用该 Key 的请求将立即失效。`)) return
    try {
      const result = await adminApi.deleteApiKey(item.id)
      toast.fromSuccess(result)
      await load()
    } catch (error) {
      toast.fromError(error)
    }
  }

  return (
    <Box>
      <Stack direction="row" sx={{ mb: 2, alignItems: 'center', justifyContent: 'space-between' }}>
        <Typography variant="body2" color="text.secondary">
          共 {items.length} 个 Key
        </Typography>
        <Button variant="contained" startIcon={<AddIcon />} onClick={() => setEditing('create')}>
          生成 Key
        </Button>
      </Stack>

      {loading ? (
        <Box sx={{ display: 'flex', justifyContent: 'center', py: 6 }}>
          <CircularProgress />
        </Box>
      ) : items.length === 0 ? (
        <Typography color="text.secondary">暂无 API Key。</Typography>
      ) : (
        <Stack spacing={1.5}>
          {items.map((item) => (
            <Box
              key={item.id}
              sx={{
                display: 'grid',
                gridTemplateColumns: { xs: '1fr auto', md: 'minmax(180px, 1fr) 2fr auto' },
                gap: 1.5,
                alignItems: 'center',
                p: 1.5,
                bgcolor: 'background.paper',
                border: '1px solid',
                borderColor: 'divider',
                borderRadius: 2,
              }}
            >
              <Box sx={{ minWidth: 0 }}>
                <Stack direction="row" spacing={1} sx={{ alignItems: 'center', mb: 0.25 }}>
                  <Typography noWrap sx={{ fontWeight: 600 }}>{item.name}</Typography>
                  <Chip size="small" label={item.enabled ? '启用' : '停用'} color={item.enabled ? 'success' : 'default'} />
                </Stack>
                <Typography variant="caption" color="text.secondary" sx={{ fontFamily: 'monospace' }}>
                  {item.key_prefix}...
                </Typography>
              </Box>

              <Stack
                direction="row"
                spacing={0.75}
                sx={{ gridColumn: { xs: '1 / -1', md: 'auto' }, flexWrap: 'wrap', rowGap: 0.75 }}
              >
                <Chip size="small" variant="outlined" label={rateLabel(item)} />
                <Chip size="small" variant="outlined" label={quotaLabel(item)} />
                <Chip size="small" variant="outlined" label={`并发 ${item.concurrency_limit ?? '不限'}`} />
                {item.allowed_ips.length ? <Chip size="small" variant="outlined" label={`IP ${item.allowed_ips.length} 条`} /> : null}
                {item.allowed_domains.length ? <Chip size="small" variant="outlined" label={`域名 ${item.allowed_domains.length} 条`} /> : null}
              </Stack>

              <Stack direction="row" spacing={0.25} sx={{ gridColumn: { xs: 2, md: 'auto' }, gridRow: { xs: 1, md: 'auto' } }}>
                <Tooltip title="重置已用额度"><IconButton size="small" onClick={() => void resetUsage(item)}><RestartAltIcon fontSize="small" /></IconButton></Tooltip>
                <Tooltip title="编辑"><IconButton size="small" onClick={() => setEditing(item)}><EditIcon fontSize="small" /></IconButton></Tooltip>
                <Tooltip title="删除"><IconButton size="small" color="error" onClick={() => void remove(item)}><DeleteIcon fontSize="small" /></IconButton></Tooltip>
              </Stack>
            </Box>
          ))}
        </Stack>
      )}

      <ApiKeyDialog
        open={editing !== null}
        apiKey={editing === 'create' || editing === null ? null : editing}
        onClose={() => setEditing(null)}
        onSaved={async (result) => {
          setEditing(null)
          if (result.key) setCreatedKey(result.key)
          await load()
        }}
      />

      <Dialog open={createdKey !== null} onClose={() => setCreatedKey(null)} fullWidth maxWidth="sm">
        <DialogTitle>API Key 已生成</DialogTitle>
        <DialogContent>
          <Alert severity="warning" sx={{ mb: 2 }}>完整 Key 只显示这一次，请立即妥善保存。</Alert>
          <TextField
            fullWidth
            value={createdKey ?? ''}
            slotProps={{ htmlInput: { readOnly: true, style: { fontFamily: 'monospace' } } }}
          />
        </DialogContent>
        <DialogActions>
          <Button
            startIcon={<ContentCopyIcon />}
            onClick={() => {
              if (createdKey) void navigator.clipboard.writeText(createdKey)
              toast.success('已复制')
            }}
          >
            复制
          </Button>
          <Button variant="contained" onClick={() => setCreatedKey(null)}>完成</Button>
        </DialogActions>
      </Dialog>
    </Box>
  )
}

function rateLabel(item: ApiKey) {
  return item.rate_limit && item.rate_window_secs
    ? `频率 ${item.rate_limit}/${item.rate_window_secs}秒`
    : '频率不限'
}

function quotaLabel(item: ApiKey) {
  return item.total_quota == null
    ? `已用 ${item.used_count} / 不限`
    : `已用 ${item.used_count} / ${item.total_quota}`
}

function ApiKeyDialog({
  open,
  apiKey,
  onClose,
  onSaved,
}: {
  open: boolean
  apiKey: ApiKey | null
  onClose: () => void
  onSaved: (result: ApiKey) => Promise<void>
}) {
  const [name, setName] = useState('')
  const [enabled, setEnabled] = useState(true)
  const [rateLimit, setRateLimit] = useState('')
  const [rateWindow, setRateWindow] = useState('')
  const [totalQuota, setTotalQuota] = useState('')
  const [concurrency, setConcurrency] = useState('')
  const [allowedIps, setAllowedIps] = useState('')
  const [allowedDomains, setAllowedDomains] = useState('')
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    if (!open) return
    setName(apiKey?.name ?? '')
    setEnabled(apiKey?.enabled ?? true)
    setRateLimit(apiKey?.rate_limit?.toString() ?? '')
    setRateWindow(apiKey?.rate_window_secs?.toString() ?? '')
    setTotalQuota(apiKey?.total_quota?.toString() ?? '')
    setConcurrency(apiKey?.concurrency_limit?.toString() ?? '')
    setAllowedIps(apiKey?.allowed_ips.join('\n') ?? '')
    setAllowedDomains(apiKey?.allowed_domains.join('\n') ?? '')
    setError(null)
  }, [apiKey, open])

  const submit = async () => {
    const positive = (raw: string, label: string): number | null => {
      if (!raw.trim()) return null
      const value = Number(raw)
      if (!Number.isSafeInteger(value) || value <= 0) throw new Error(`${label}必须是正整数`)
      return value
    }
    try {
      if (!name.trim()) throw new Error('请填写名称')
      const rate = positive(rateLimit, '频率次数')
      const window = positive(rateWindow, '时间窗口')
      if ((rate == null) !== (window == null)) throw new Error('频率次数与时间窗口必须同时填写')
      const split = (raw: string) => raw.split(/[\n,]/).map((item) => item.trim()).filter(Boolean)
      const body: ApiKeyWrite = {
        name: name.trim(),
        enabled,
        rate_limit: rate,
        rate_window_secs: window,
        total_quota: positive(totalQuota, '总额度'),
        concurrency_limit: positive(concurrency, '并发上限'),
        allowed_ips: split(allowedIps),
        allowed_domains: split(allowedDomains),
      }
      setBusy(true)
      setError(null)
      const result = apiKey
        ? await adminApi.updateApiKey(apiKey.id, body)
        : await adminApi.createApiKey(body)
      await onSaved(result)
    } catch (caught) {
      setError(caught instanceof ApiError || caught instanceof Error ? caught.message : '保存失败')
    } finally {
      setBusy(false)
    }
  }

  return (
    <Dialog open={open} onClose={onClose} fullWidth maxWidth="sm">
      <DialogTitle>{apiKey ? '编辑 API Key' : '生成 API Key'}</DialogTitle>
      <DialogContent>
        <Stack spacing={2} sx={{ mt: 1 }}>
          {error ? <Alert severity="error" onClose={() => setError(null)}>{error}</Alert> : null}
          <TextField label="名称" required value={name} onChange={(event) => setName(event.target.value)} slotProps={{ htmlInput: { maxLength: 128 } }} />
          <FormControlLabel control={<Switch checked={enabled} onChange={(_, value) => setEnabled(value)} />} label="启用" />
          <Stack direction={{ xs: 'column', sm: 'row' }} spacing={2}>
            <TextField label="频率次数" type="number" value={rateLimit} onChange={(event) => setRateLimit(event.target.value)} fullWidth placeholder="不限" slotProps={{ htmlInput: { min: 1 } }} />
            <TextField label="时间窗口（秒）" type="number" value={rateWindow} onChange={(event) => setRateWindow(event.target.value)} fullWidth placeholder="不限" slotProps={{ htmlInput: { min: 1 } }} />
          </Stack>
          <Stack direction={{ xs: 'column', sm: 'row' }} spacing={2}>
            <TextField label="总额度" type="number" value={totalQuota} onChange={(event) => setTotalQuota(event.target.value)} fullWidth placeholder="不限" slotProps={{ htmlInput: { min: 1 } }} />
            <TextField label="并发上限" type="number" value={concurrency} onChange={(event) => setConcurrency(event.target.value)} fullWidth placeholder="不限" slotProps={{ htmlInput: { min: 1 } }} />
          </Stack>
          <TextField label="允许的 IP / CIDR" value={allowedIps} onChange={(event) => setAllowedIps(event.target.value)} multiline minRows={2} placeholder={'留空表示不限\n127.0.0.1\n10.0.0.0/8'} helperText="每行或逗号分隔，支持精确 IP 和 CIDR。" />
          <TextField label="允许的来源域名" value={allowedDomains} onChange={(event) => setAllowedDomains(event.target.value)} multiline minRows={2} placeholder={'留空表示不限\nexample.com\n*.example.com'} helperText="只填写域名，不含协议或端口；*.example.com 仅匹配子域。" />
        </Stack>
      </DialogContent>
      <DialogActions>
        <Button onClick={onClose}>取消</Button>
        <Button variant="contained" disabled={busy} onClick={() => void submit()}>保存</Button>
      </DialogActions>
    </Dialog>
  )
}
