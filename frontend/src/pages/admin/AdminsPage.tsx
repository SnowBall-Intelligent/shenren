import { useCallback, useEffect, useState } from 'react'
import { useOutletContext } from 'react-router-dom'
import {
  Alert,
  Box,
  Button,
  CircularProgress,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  IconButton,
  Stack,
  TextField,
  Typography,
} from '@mui/material'
import DeleteIcon from '@mui/icons-material/Delete'
import { adminApi, normalizeAdmins } from '../../api'
import type { Admin, AdminMe } from '../../api/types'
import { ApiError } from '../../api/client'

export default function AdminsPage() {
  const { me } = useOutletContext<{ me: AdminMe }>()
  const [admins, setAdmins] = useState<Admin[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [open, setOpen] = useState(false)

  const load = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      const data = await adminApi.listAdmins()
      setAdmins(normalizeAdmins(data))
    } catch (e) {
      setError(e instanceof ApiError ? e.message : '加载失败')
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    void load()
  }, [load])

  const handleDelete = async (admin: Admin) => {
    if (admin.id === me.id && admins.length <= 1) {
      setError('不能删除唯一的管理员')
      return
    }
    if (!window.confirm(`确定删除管理员「${admin.username}」？`)) return
    try {
      await adminApi.deleteAdmin(admin.id)
      await load()
    } catch (e) {
      setError(e instanceof ApiError ? e.message : '删除失败')
    }
  }

  return (
    <Box>
      <Stack direction="row" sx={{ mb: 2, justifyContent: 'space-between', alignItems: 'center' }}>
        <Typography variant="body2" color="text.secondary">
          共 {admins.length} 位管理员
        </Typography>
        <Button variant="contained" onClick={() => setOpen(true)}>
          新增管理员
        </Button>
      </Stack>

      {error ? (
        <Alert severity="error" sx={{ mb: 2 }} onClose={() => setError(null)}>
          {error}
        </Alert>
      ) : null}

      {loading ? (
        <Box sx={{ display: 'flex', justifyContent: 'center', py: 6 }}>
          <CircularProgress />
        </Box>
      ) : (
        <Stack spacing={1.5}>
          {admins.map((a) => (
            <Box
              key={a.id}
              sx={{
                display: 'flex',
                alignItems: 'center',
                gap: 1.5,
                p: 1.5,
                bgcolor: 'background.paper',
                borderRadius: 2,
                border: '1px solid #2a2a2a',
              }}
            >
              <Box sx={{ flex: 1 }}>
                <Typography>
                  {a.username}
                  {a.id === me.id ? (
                    <Typography component="span" variant="caption" color="text.secondary" sx={{ ml: 1 }}>
                      （当前）
                    </Typography>
                  ) : null}
                </Typography>
                <Typography variant="caption" color="text.secondary">
                  {formatTime(a.created_at)}
                </Typography>
              </Box>
              <IconButton
                size="small"
                color="error"
                disabled={admins.length <= 1}
                onClick={() => void handleDelete(a)}
                aria-label="删除"
              >
                <DeleteIcon fontSize="small" />
              </IconButton>
            </Box>
          ))}
        </Stack>
      )}

      <CreateAdminDialog
        open={open}
        onClose={() => setOpen(false)}
        onCreated={async () => {
          setOpen(false)
          await load()
        }}
        onError={(msg) => setError(msg)}
      />
    </Box>
  )
}

function formatTime(iso: string) {
  try {
    return new Date(iso).toLocaleString('zh-CN')
  } catch {
    return iso
  }
}

function CreateAdminDialog({
  open,
  onClose,
  onCreated,
  onError,
}: {
  open: boolean
  onClose: () => void
  onCreated: () => Promise<void>
  onError: (msg: string) => void
}) {
  const [username, setUsername] = useState('')
  const [password, setPassword] = useState('')
  const [busy, setBusy] = useState(false)

  useEffect(() => {
    if (open) {
      setUsername('')
      setPassword('')
    }
  }, [open])

  const submit = async () => {
    if (!username.trim() || password.length < 6) {
      onError('用户名必填，密码至少 6 位')
      return
    }
    setBusy(true)
    try {
      await adminApi.createAdmin(username.trim(), password)
      await onCreated()
    } catch (e) {
      onError(e instanceof ApiError ? e.message : '创建失败')
    } finally {
      setBusy(false)
    }
  }

  return (
    <Dialog open={open} onClose={onClose} fullWidth maxWidth="xs">
      <DialogTitle>新增管理员</DialogTitle>
      <DialogContent>
        <Stack spacing={2} sx={{ mt: 1 }}>
          <TextField
            label="用户名"
            value={username}
            onChange={(e) => setUsername(e.target.value)}
            autoComplete="off"
            fullWidth
          />
          <TextField
            label="密码"
            type="password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            autoComplete="new-password"
            fullWidth
          />
        </Stack>
      </DialogContent>
      <DialogActions>
        <Button onClick={onClose}>取消</Button>
        <Button variant="contained" disabled={busy} onClick={() => void submit()}>
          创建
        </Button>
      </DialogActions>
    </Dialog>
  )
}
