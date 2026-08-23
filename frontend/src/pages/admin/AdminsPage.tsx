import { useCallback, useEffect, useId, useState } from 'react'
import { useOutletContext } from 'react-router-dom'
import {
  Box,
  Button,
  Chip,
  CircularProgress,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  FormControl,
  FormHelperText,
  IconButton,
  InputLabel,
  MenuItem,
  Select,
  Stack,
  TextField,
  Tooltip,
  Typography,
} from '@mui/material'
import DeleteIcon from '@mui/icons-material/Delete'
import EditOutlinedIcon from '@mui/icons-material/EditOutlined'
import { adminApi, normalizeAdmins } from '../../api'
import type { Admin, AdminMe, AdminRole } from '../../api/types'
import { ApiError } from '../../api/client'
import { useToast } from '../../components/AppToast'

const ROLE_LABELS: Record<AdminRole, string> = {
  super_admin: '超级管理员',
  admin: '普通管理员',
}

const ROLE_DESCRIPTIONS: Record<AdminRole, string> = {
  super_admin: '拥有全部后台管理权限，包括账号、API Key 与系统设置。',
  admin: '可完整管理言论和神人，不能访问其他后台模块。',
}

export default function AdminsPage() {
  const { me } = useOutletContext<{ me: AdminMe }>()
  const toast = useToast()
  const [admins, setAdmins] = useState<Admin[]>([])
  const [loading, setLoading] = useState(true)
  const [open, setOpen] = useState(false)
  const [editing, setEditing] = useState<Admin | null>(null)

  const load = useCallback(async () => {
    setLoading(true)
    try {
      const data = await adminApi.listAdmins()
      setAdmins(normalizeAdmins(data))
    } catch (e) {
      toast.fromError(e)
    } finally {
      setLoading(false)
    }
  }, [toast])

  useEffect(() => {
    void load()
  }, [load])

  const superAdminCount = admins.filter((admin) => admin.role === 'super_admin').length

  const handleDelete = async (admin: Admin) => {
    if (admin.id === me.id) {
      toast.error('不能删除自己的账号')
      return
    }
    if (admin.role === 'super_admin' && superAdminCount <= 1) {
      toast.error('不能删除最后一名超级管理员')
      return
    }
    if (!window.confirm(`确定删除管理员「${admin.username}」？`)) return
    try {
      await adminApi.deleteAdmin(admin.id)
      toast.success('管理员已删除')
      await load()
    } catch (e) {
      toast.fromError(e)
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

      {loading ? (
        <Box sx={{ display: 'flex', justifyContent: 'center', py: 6 }}>
          <CircularProgress />
        </Box>
      ) : (
        <Stack spacing={1.5}>
          {admins.map((admin) => {
            const isSelf = admin.id === me.id
            const isLastSuperAdmin = admin.role === 'super_admin' && superAdminCount <= 1
            const editDisabled = isSelf || isLastSuperAdmin
            const editHint = isSelf
              ? '不能修改自己的角色'
              : isLastSuperAdmin
                ? '必须保留至少一名超级管理员'
                : '修改角色'
            const deleteHint = isSelf
              ? '不能删除自己的账号'
              : isLastSuperAdmin
                ? '必须保留至少一名超级管理员'
                : '删除管理员'

            return (
              <Box
                key={admin.id}
                sx={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: 1.5,
                  p: 1.5,
                  bgcolor: 'background.paper',
                  borderRadius: 2,
                  border: '1px solid',
                  borderColor: 'divider',
                }}
              >
                <Box sx={{ flex: 1, minWidth: 0 }}>
                  <Stack direction="row" spacing={1} sx={{ alignItems: 'center', flexWrap: 'wrap' }}>
                    <Typography sx={{ overflowWrap: 'anywhere' }}>
                      {admin.username}
                      {isSelf ? (
                        <Typography
                          component="span"
                          variant="caption"
                          color="text.secondary"
                          sx={{ ml: 1 }}
                        >
                          （当前）
                        </Typography>
                      ) : null}
                    </Typography>
                    <Chip
                      size="small"
                      label={ROLE_LABELS[admin.role]}
                      color={admin.role === 'super_admin' ? 'primary' : 'default'}
                      variant={admin.role === 'super_admin' ? 'filled' : 'outlined'}
                    />
                  </Stack>
                  <Typography variant="caption" color="text.secondary">
                    {formatTime(admin.created_at)}
                  </Typography>
                </Box>
                <Tooltip title={editHint}>
                  <span>
                    <IconButton
                      size="small"
                      disabled={editDisabled}
                      onClick={() => setEditing(admin)}
                      aria-label={`编辑${admin.username}角色`}
                    >
                      <EditOutlinedIcon fontSize="small" />
                    </IconButton>
                  </span>
                </Tooltip>
                <Tooltip title={deleteHint}>
                  <span>
                    <IconButton
                      size="small"
                      color="error"
                      disabled={isSelf || isLastSuperAdmin}
                      onClick={() => void handleDelete(admin)}
                      aria-label={`删除${admin.username}`}
                    >
                      <DeleteIcon fontSize="small" />
                    </IconButton>
                  </span>
                </Tooltip>
              </Box>
            )
          })}
        </Stack>
      )}

      <CreateAdminDialog
        open={open}
        onClose={() => setOpen(false)}
        onCreated={async () => {
          setOpen(false)
          toast.success('管理员已创建')
          await load()
        }}
        onError={(message) => toast.error(message)}
      />
      <EditRoleDialog
        admin={editing}
        onClose={() => setEditing(null)}
        onUpdated={async () => {
          setEditing(null)
          toast.success('角色已更新')
          await load()
        }}
        onError={(message) => toast.error(message)}
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

function RoleField({ role, onChange }: { role: AdminRole; onChange: (role: AdminRole) => void }) {
  const labelId = useId()
  return (
    <FormControl fullWidth>
      <InputLabel id={labelId}>角色</InputLabel>
      <Select
        labelId={labelId}
        label="角色"
        value={role}
        onChange={(event) => onChange(event.target.value as AdminRole)}
      >
        <MenuItem value="admin">普通管理员</MenuItem>
        <MenuItem value="super_admin">超级管理员</MenuItem>
      </Select>
      <FormHelperText>{ROLE_DESCRIPTIONS[role]}</FormHelperText>
    </FormControl>
  )
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
  onError: (message: string) => void
}) {
  const [username, setUsername] = useState('')
  const [password, setPassword] = useState('')
  const [role, setRole] = useState<AdminRole>('admin')
  const [busy, setBusy] = useState(false)

  useEffect(() => {
    if (open) {
      setUsername('')
      setPassword('')
      setRole('admin')
    }
  }, [open])

  const submit = async () => {
    if (!username.trim() || password.length < 6) {
      onError('用户名必填，密码至少 6 位')
      return
    }
    setBusy(true)
    try {
      await adminApi.createAdmin(username.trim(), password, role)
      await onCreated()
    } catch (error) {
      onError(error instanceof ApiError ? error.message : '创建失败')
    } finally {
      setBusy(false)
    }
  }

  return (
    <Dialog open={open} onClose={busy ? undefined : onClose} fullWidth maxWidth="xs">
      <DialogTitle>新增管理员</DialogTitle>
      <DialogContent>
        <Stack spacing={2} sx={{ mt: 1 }}>
          <TextField
            label="用户名"
            value={username}
            onChange={(event) => setUsername(event.target.value)}
            autoComplete="off"
            autoFocus
            fullWidth
          />
          <TextField
            label="密码"
            type="password"
            value={password}
            onChange={(event) => setPassword(event.target.value)}
            autoComplete="new-password"
            fullWidth
          />
          <RoleField role={role} onChange={setRole} />
        </Stack>
      </DialogContent>
      <DialogActions>
        <Button disabled={busy} onClick={onClose}>
          取消
        </Button>
        <Button variant="contained" disabled={busy} onClick={() => void submit()}>
          创建
        </Button>
      </DialogActions>
    </Dialog>
  )
}

function EditRoleDialog({
  admin,
  onClose,
  onUpdated,
  onError,
}: {
  admin: Admin | null
  onClose: () => void
  onUpdated: () => Promise<void>
  onError: (message: string) => void
}) {
  const [role, setRole] = useState<AdminRole>('admin')
  const [busy, setBusy] = useState(false)

  useEffect(() => {
    if (admin) setRole(admin.role)
  }, [admin])

  const submit = async () => {
    if (!admin || role === admin.role) return
    setBusy(true)
    try {
      await adminApi.updateAdminRole(admin.id, role)
      await onUpdated()
    } catch (error) {
      onError(error instanceof ApiError ? error.message : '角色更新失败')
    } finally {
      setBusy(false)
    }
  }

  return (
    <Dialog open={Boolean(admin)} onClose={busy ? undefined : onClose} fullWidth maxWidth="xs">
      <DialogTitle>编辑管理员角色</DialogTitle>
      <DialogContent>
        <Stack spacing={2} sx={{ mt: 1 }}>
          <Typography variant="body2" color="text.secondary">
            {admin?.username}
          </Typography>
          <RoleField role={role} onChange={setRole} />
        </Stack>
      </DialogContent>
      <DialogActions>
        <Button disabled={busy} onClick={onClose}>
          取消
        </Button>
        <Button
          variant="contained"
          disabled={busy || !admin || role === admin.role}
          onClick={() => void submit()}
        >
          保存
        </Button>
      </DialogActions>
    </Dialog>
  )
}
