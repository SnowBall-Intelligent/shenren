import { useCallback, useEffect, useState } from 'react'
import {
  Alert,
  Avatar,
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
import EditIcon from '@mui/icons-material/Edit'
import { adminApi, normalizePersons } from '../../api'
import type { Person } from '../../api/types'
import { ApiError, nameInitial, uploadUrl } from '../../api/client'
import QuoteCreateDialog from './QuoteCreateDialog'

export default function PersonsPage() {
  const [persons, setPersons] = useState<Person[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [dialog, setDialog] = useState<'create' | Person | null>(null)
  const [quotePerson, setQuotePerson] = useState<Person | null>(null)

  const load = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      const data = await adminApi.listPersons()
      setPersons(normalizePersons(data))
    } catch (e) {
      setError(e instanceof ApiError ? e.message : '加载失败')
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    void load()
  }, [load])

  const handleDelete = async (id: number) => {
    if (!window.confirm('确定删除该神人？')) return
    try {
      await adminApi.deletePerson(id)
      await load()
    } catch (e) {
      setError(e instanceof ApiError ? e.message : '删除失败')
    }
  }

  return (
    <Box>
      <Stack direction="row" sx={{ mb: 2, justifyContent: 'space-between', alignItems: 'center' }}>
        <Typography variant="body2" color="text.secondary">
          共 {persons.length} 位神人
        </Typography>
        <Button variant="contained" onClick={() => setDialog('create')}>
          新增神人
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
      ) : persons.length === 0 ? (
        <Typography color="text.secondary">暂无神人，请先新增。</Typography>
      ) : (
        <Stack spacing={1.5}>
          {persons.map((p) => (
            <Box
              key={p.id}
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
              <Avatar src={uploadUrl(p.avatar_url)} alt={p.name}>
                {nameInitial(p.name)}
              </Avatar>
              <Typography sx={{ flex: 1 }}>{p.name}</Typography>
              <Button size="small" onClick={() => setQuotePerson(p)}>
                添加语录
              </Button>
              <IconButton size="small" onClick={() => setDialog(p)} aria-label="编辑">
                <EditIcon fontSize="small" />
              </IconButton>
              <IconButton size="small" color="error" onClick={() => void handleDelete(p.id)} aria-label="删除">
                <DeleteIcon fontSize="small" />
              </IconButton>
            </Box>
          ))}
        </Stack>
      )}

      <PersonDialog
        open={dialog !== null}
        person={dialog === 'create' || dialog === null ? null : dialog}
        onClose={() => setDialog(null)}
        onSaved={async () => {
          setDialog(null)
          await load()
        }}
      />
      <QuoteCreateDialog
        open={quotePerson !== null}
        person={quotePerson}
        onClose={() => setQuotePerson(null)}
        onCreated={() => setQuotePerson(null)}
      />
    </Box>
  )
}

function PersonDialog({
  open,
  person,
  onClose,
  onSaved,
}: {
  open: boolean
  person: Person | null
  onClose: () => void
  onSaved: () => Promise<void>
}) {
  const [name, setName] = useState('')
  const [avatar, setAvatar] = useState<File | null>(null)
  const [avatarUrl, setAvatarUrl] = useState('')
  const [previewObjectUrl, setPreviewObjectUrl] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  const [formError, setFormError] = useState<string | null>(null)

  useEffect(() => {
    if (open) {
      setName(person?.name ?? '')
      setAvatar(null)
      setAvatarUrl('')
      setFormError(null)
    }
  }, [open, person])

  useEffect(() => {
    if (!avatar) {
      setPreviewObjectUrl(null)
      return
    }
    const url = URL.createObjectURL(avatar)
    setPreviewObjectUrl(url)
    return () => URL.revokeObjectURL(url)
  }, [avatar])

  const previewSrc =
    previewObjectUrl ??
    (avatarUrl.trim() || (person ? uploadUrl(person.avatar_url) : undefined) || undefined)

  const submit = async () => {
    const trimmed = name.trim()
    const url = avatarUrl.trim()
    if (!trimmed) {
      setFormError('请填写名称')
      return
    }
    if (url && !avatar && !/^https?:\/\//i.test(url)) {
      setFormError('头像 URL 须以 http:// 或 https:// 开头')
      return
    }
    setBusy(true)
    setFormError(null)
    try {
      const form = new FormData()
      form.append('name', trimmed)
      if (avatar) form.append('avatar', avatar)
      else if (url) form.append('avatar_url', url)
      if (person) {
        await adminApi.updatePerson(person.id, form)
      } else {
        await adminApi.createPerson(form)
      }
      await onSaved()
    } catch (e) {
      setFormError(e instanceof ApiError ? e.message : '保存失败')
    } finally {
      setBusy(false)
    }
  }

  return (
    <Dialog open={open} onClose={onClose} fullWidth maxWidth="sm">
      <DialogTitle>{person ? '编辑神人' : '新增神人'}</DialogTitle>
      <DialogContent>
        <Stack spacing={2} sx={{ mt: 1 }}>
          {formError ? (
            <Alert severity="error" onClose={() => setFormError(null)}>
              {formError}
            </Alert>
          ) : null}
          <TextField
            label="名称"
            value={name}
            onChange={(e) => setName(e.target.value)}
            fullWidth
            required
          />
          <Button variant="outlined" component="label">
            {avatar ? avatar.name : person ? '更换头像（可选）' : '上传头像（可选）'}
            <input
              type="file"
              hidden
              accept="image/*"
              onChange={(e) => {
                setAvatar(e.target.files?.[0] ?? null)
                if (e.target.files?.[0]) setAvatarUrl('')
              }}
            />
          </Button>
          <TextField
            label="头像 URL（可选）"
            value={avatarUrl}
            onChange={(e) => {
              setAvatarUrl(e.target.value)
              if (e.target.value.trim()) setAvatar(null)
            }}
            fullWidth
            placeholder="https://"
            helperText="也可上传文件。都不填时将使用名称首字生成头像。"
          />
          <Box sx={{ display: 'flex', justifyContent: 'center' }}>
            <Avatar src={previewSrc} alt={name} sx={{ width: 64, height: 64 }}>
              {nameInitial(name)}
            </Avatar>
          </Box>
        </Stack>
      </DialogContent>
      <DialogActions>
        <Button onClick={onClose}>取消</Button>
        <Button variant="contained" disabled={busy} onClick={() => void submit()}>
          保存
        </Button>
      </DialogActions>
    </Dialog>
  )
}
