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
  TablePagination,
  TextField,
  Typography,
} from '@mui/material'
import DeleteIcon from '@mui/icons-material/Delete'
import EditIcon from '@mui/icons-material/Edit'
import { adminApi } from '../../api'
import type { Person } from '../../api/types'
import { ApiError, nameInitial, qqAvatarUrl, uploadUrl } from '../../api/client'
import QuoteCreateDialog from './QuoteCreateDialog'
import { useToast } from '../../components/AppToast'

export default function PersonsPage() {
  const toast = useToast()
  const [persons, setPersons] = useState<Person[]>([])
  const [page, setPage] = useState(1)
  const [pageSize, setPageSize] = useState(20)
  const [total, setTotal] = useState(0)
  const [loading, setLoading] = useState(true)
  const [dialog, setDialog] = useState<'create' | Person | null>(null)
  const [quotePerson, setQuotePerson] = useState<Person | null>(null)

  const load = useCallback(async (p = 1, size = pageSize) => {
    setLoading(true)
    try {
      const data = await adminApi.listPersons({ page: p, page_size: size })
      setPersons(data.items)
      setTotal(data.total)
      setPage(data.page)
      setPageSize(data.page_size)
    } catch (e) {
      toast.fromError(e)
    } finally {
      setLoading(false)
    }
  }, [pageSize, toast])

  useEffect(() => {
    void load(1)
  }, [load])

  const handleDelete = async (id: number) => {
    if (!window.confirm('确定删除该神人？')) return
    try {
      await adminApi.deletePerson(id)
      await load(page)
    } catch (e) {
      toast.fromError(e)
    }
  }

  return (
    <Box>
      <Stack direction="row" sx={{ mb: 2, justifyContent: 'space-between', alignItems: 'center' }}>
        <Typography variant="body2" color="text.secondary">
          共 {total} 位神人
        </Typography>
        <Button variant="contained" onClick={() => setDialog('create')}>
          新增神人
        </Button>
      </Stack>

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
                border: '1px solid',
                borderColor: 'divider',
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

      <TablePagination
        component="div"
        count={total}
        page={Math.max(0, page - 1)}
        onPageChange={(_, next) => void load(next + 1, pageSize)}
        rowsPerPage={pageSize}
        onRowsPerPageChange={(e) => {
          const size = parseInt(e.target.value, 10)
          setPageSize(size)
          void load(1, size)
        }}
        rowsPerPageOptions={[10, 20, 50]}
        labelRowsPerPage="每页"
        labelDisplayedRows={({ from, to, count }) =>
          `${from}–${to} / ${count === -1 ? `超过 ${to}` : count}`
        }
      />

      <PersonDialog
        open={dialog !== null}
        person={dialog === 'create' || dialog === null ? null : dialog}
        onClose={() => setDialog(null)}
        onSaved={async () => {
          setDialog(null)
          await load(page)
        }}
      />
      <QuoteCreateDialog
        open={quotePerson !== null}
        person={quotePerson}
        persons={persons}
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
  const [qq, setQq] = useState('')
  const [previewObjectUrl, setPreviewObjectUrl] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  const [formError, setFormError] = useState<string | null>(null)

  useEffect(() => {
    if (open) {
      setName(person?.name ?? '')
      setAvatar(null)
      setAvatarUrl('')
      setQq('')
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
    (qqAvatarUrl(qq) || avatarUrl.trim() || (person ? uploadUrl(person.avatar_url) : undefined) || undefined)

  const submit = async () => {
    const trimmed = name.trim()
    const url = avatarUrl.trim()
    const qqValue = qq.trim()
    if (!trimmed) {
      setFormError('请填写名称')
      return
    }
    if (url && !avatar && !/^https?:\/\//i.test(url)) {
      setFormError('头像 URL 须以 http:// 或 https:// 开头')
      return
    }
    if (qqValue && !qqAvatarUrl(qqValue)) {
      setFormError('请输入 5-20 位、首位不为 0 的 QQ 号')
      return
    }
    setBusy(true)
    setFormError(null)
    try {
      const form = new FormData()
      form.append('name', trimmed)
      if (avatar) form.append('avatar', avatar)
      else if (qqValue) form.append('qq', qqValue)
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
            slotProps={{ htmlInput: { maxLength: 128 } }}
          />
          <Button variant="outlined" component="label">
            {avatar ? avatar.name : person ? '更换头像（可选）' : '上传头像（可选）'}
            <input
              type="file"
              hidden
              accept="image/*"
              onChange={(e) => {
                setAvatar(e.target.files?.[0] ?? null)
                if (e.target.files?.[0]) {
                  setAvatarUrl('')
                  setQq('')
                }
              }}
            />
          </Button>
          <TextField
            label="QQ 号获取头像（可选）"
            value={qq}
            onChange={(e) => {
              setQq(e.target.value.replace(/\D/g, '').slice(0, 20))
              if (e.target.value.trim()) {
                setAvatar(null)
                setAvatarUrl('')
              }
            }}
            fullWidth
            placeholder="输入 QQ 号"
            helperText="将保存 QQ 头像 CDN 链接，不保存 QQ 号。"
            slotProps={{ htmlInput: { inputMode: 'numeric' } }}
          />
          <TextField
            label="头像 URL（可选）"
            value={avatarUrl}
            onChange={(e) => {
              setAvatarUrl(e.target.value)
              if (e.target.value.trim()) {
                setAvatar(null)
                setQq('')
              }
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
