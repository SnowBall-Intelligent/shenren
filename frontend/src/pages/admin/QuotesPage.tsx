import { useCallback, useEffect, useState } from 'react'
import {
  Alert,
  Autocomplete,
  Avatar,
  Box,
  Button,
  CircularProgress,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  FormControl,
  InputLabel,
  MenuItem,
  Select,
  Stack,
  TablePagination,
  TextField,
  Typography,
  IconButton,
} from '@mui/material'
import EditIcon from '@mui/icons-material/Edit'
import DeleteIcon from '@mui/icons-material/Delete'
import { adminApi, normalizePersons, publicApi } from '../../api'
import type { Person, Quote } from '../../api/types'
import { ApiError, nameInitial, uploadUrl } from '../../api/client'
import QuoteMarkdown from '../../components/QuoteMarkdown'
import QuoteCreateDialog from './QuoteCreateDialog'
import { useToast } from '../../components/AppToast'

export default function QuotesPage({ variant }: { variant: 'review' | 'list' }) {
  const toast = useToast()
  const isReview = variant === 'review'
  const [quotes, setQuotes] = useState<Quote[]>([])
  const [page, setPage] = useState(1)
  const [pageSize, setPageSize] = useState(20)
  const [total, setTotal] = useState(0)
  const [loading, setLoading] = useState(true)
  const [persons, setPersons] = useState<Person[]>([])
  const [approveTarget, setApproveTarget] = useState<Quote | null>(null)
  const [createOpen, setCreateOpen] = useState(false)
  const [editQuote, setEditQuote] = useState<Quote | null>(null)

  const load = useCallback(async (p = 1, size = pageSize) => {
    setLoading(true)
    try {
      const data = await adminApi.listQuotes({
        status: isReview ? 'unapproved' : 'approved',
        page: p,
        page_size: size,
      })
      setQuotes(data.items)
      setTotal(data.total)
      setPage(data.page)
      setPageSize(data.page_size)
    } catch (e) {
      toast.fromError(e)
    } finally {
      setLoading(false)
    }
  }, [isReview, pageSize, toast])

  useEffect(() => {
    void load(1)
  }, [load])

  useEffect(() => {
    publicApi
      .getPersons()
      .then((d) => setPersons(normalizePersons(d)))
      .catch(() => setPersons([]))
  }, [])

  const handleReject = async (id: number) => {
    try {
      await adminApi.rejectQuote(id)
      await load(page)
    } catch (e) {
      toast.fromError(e)
    }
  }

  const handleApproveSimple = async (quote: Quote) => {
    if (quote.person_id == null && quote.proposed_person_name) {
      setApproveTarget(quote)
      return
    }
    try {
      await adminApi.approveQuote(quote.id)
      await load(page)
    } catch (e) {
      toast.fromError(e)
    }
  }

  const handleDelete = async (quote: Quote) => {
    if (!window.confirm('确定删除这条语录？')) return
    try {
      const data = await adminApi.deleteQuote(quote.id)
      toast.fromSuccess(data)
      await load(page)
    } catch (e) {
      toast.fromError(e)
    }
  }

  return (
    <Box>
      {!isReview ? (
        <Stack direction="row" sx={{ mb: 2, justifyContent: 'flex-end' }}>
          <Button variant="contained" onClick={() => setCreateOpen(true)}>
            添加语录
          </Button>
        </Stack>
      ) : null}

      {loading ? (
        <Box sx={{ display: 'flex', justifyContent: 'center', py: 6 }}>
          <CircularProgress />
        </Box>
      ) : quotes.length === 0 ? (
        <Typography color="text.secondary">暂无记录</Typography>
      ) : (
        <Stack spacing={2}>
          {quotes.map((q) => (
            <Box
              key={q.id}
              sx={{
                p: 2,
                bgcolor: 'background.paper',
                borderRadius: 2,
                border: '1px solid #2a2a2a',
              }}
            >
              <Stack direction="row" spacing={1} sx={{ mb: 1, alignItems: 'center', flexWrap: 'wrap' }}>
                <Typography variant="body2" color="text.secondary">
                  {q.person?.name ?? q.proposed_person_name ?? '（无神人）'}
                  {q.proposed_person_name && !q.person_id ? ' · 新神人提案' : ''}
                  {isReview ? ` · ${q.status === 'rejected' ? '已驳回' : '待审'}` : ''}
                </Typography>
                <Typography variant="caption" color="text.secondary" sx={{ ml: 'auto' }}>
                  {formatTime(q.created_at)}
                </Typography>
              </Stack>
              <Box sx={{ mb: 1 }}>
                <QuoteMarkdown content={q.content} />
              </Box>
              {q.source ? (
                <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mb: 1 }}>
                  来源：{q.source}
                </Typography>
              ) : null}
              {isReview && q.status === 'pending' ? (
                <Stack direction="row" spacing={1}>
                  <Button size="small" variant="contained" onClick={() => void handleApproveSimple(q)}>
                    通过
                  </Button>
                  <Button size="small" color="error" variant="outlined" onClick={() => void handleReject(q.id)}>
                    驳回
                  </Button>
                </Stack>
              ) : null}
              {!isReview ? (
                <Stack direction="row" spacing={0.5} justifyContent="flex-end">
                  <IconButton size="small" onClick={() => setEditQuote(q)} aria-label="修改">
                    <EditIcon fontSize="small" />
                  </IconButton>
                  <IconButton size="small" color="error" onClick={() => void handleDelete(q)} aria-label="删除">
                    <DeleteIcon fontSize="small" />
                  </IconButton>
                </Stack>
              ) : null}
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

      <ApproveDialog
        quote={approveTarget}
        persons={persons}
        onClose={() => setApproveTarget(null)}
        onDone={async () => {
          setApproveTarget(null)
          await load(page)
        }}
      />
      <QuoteCreateDialog
        open={createOpen}
        persons={persons}
        onClose={() => setCreateOpen(false)}
        onCreated={async () => {
          setCreateOpen(false)
          await load(1)
        }}
      />
      <QuoteCreateDialog
        open={editQuote !== null}
        quote={editQuote}
        persons={persons}
        onClose={() => setEditQuote(null)}
        onCreated={async () => {
          setEditQuote(null)
          await load(page)
        }}
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

function ApproveDialog({
  quote,
  persons,
  onClose,
  onDone,
}: {
  quote: Quote | null
  persons: Person[]
  onClose: () => void
  onDone: () => Promise<void>
}) {
  const [mode, setMode] = useState<'bind' | 'create'>('create')
  const [person, setPerson] = useState<Person | null>(null)
  const [name, setName] = useState('')
  const [avatar, setAvatar] = useState<File | null>(null)
  const [avatarUrl, setAvatarUrl] = useState('')
  const [previewObjectUrl, setPreviewObjectUrl] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  const [formError, setFormError] = useState<string | null>(null)

  useEffect(() => {
    if (quote) {
      setName(quote.proposed_person_name ?? '')
      setPerson(null)
      setAvatar(null)
      setAvatarUrl('')
      setFormError(null)
      setMode('create')
    }
  }, [quote])

  useEffect(() => {
    if (!avatar) {
      setPreviewObjectUrl(null)
      return
    }
    const url = URL.createObjectURL(avatar)
    setPreviewObjectUrl(url)
    return () => URL.revokeObjectURL(url)
  }, [avatar])

  const previewSrc = previewObjectUrl ?? (avatarUrl.trim() || undefined)

  const submit = async () => {
    if (!quote) return
    setFormError(null)
    if (mode === 'bind') {
      if (!person) {
        setFormError('请选择要绑定的神人')
        return
      }
    } else if (!name.trim()) {
      setFormError('请填写神人名称')
      return
    } else {
      const url = avatarUrl.trim()
      if (url && !avatar && !/^https?:\/\//i.test(url)) {
        setFormError('头像 URL 须以 http:// 或 https:// 开头')
        return
      }
    }
    setBusy(true)
    try {
      if (mode === 'bind') {
        await adminApi.approveQuote(quote.id, { person_id: person!.id })
      } else if (avatar) {
        const form = new FormData()
        form.append('create_person_name', name.trim())
        form.append('avatar', avatar)
        await adminApi.approveQuoteWithAvatar(quote.id, form)
      } else {
        await adminApi.approveQuote(quote.id, {
          create_person_name: name.trim(),
          avatar_url: avatarUrl.trim() || undefined,
        })
      }
      await onDone()
    } catch (e) {
      setFormError(e instanceof ApiError ? e.message : '通过失败')
    } finally {
      setBusy(false)
    }
  }

  return (
    <Dialog open={!!quote} onClose={onClose} fullWidth maxWidth="sm">
      <DialogTitle>审核通过 · 处理新神人</DialogTitle>
      <DialogContent>
        {formError ? (
          <Alert severity="error" sx={{ mb: 2 }} onClose={() => setFormError(null)}>
            {formError}
          </Alert>
        ) : null}
        <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
          提案名称：{quote?.proposed_person_name}
        </Typography>
        <FormControl fullWidth sx={{ mb: 2 }}>
          <InputLabel id="approve-mode">处理方式</InputLabel>
          <Select
            labelId="approve-mode"
            label="处理方式"
            value={mode}
            onChange={(e) => setMode(e.target.value as 'bind' | 'create')}
          >
            <MenuItem value="create">创建新神人</MenuItem>
            <MenuItem value="bind">绑定到已有神人</MenuItem>
          </Select>
        </FormControl>
        {mode === 'bind' ? (
          <Autocomplete
            options={persons}
            value={person}
            onChange={(_, v) => setPerson(v)}
            getOptionLabel={(o) => o.name}
            isOptionEqualToValue={(a, b) => a.id === b.id}
            renderOption={(props, option) => {
              const { key, ...rest } = props
              return (
                <li key={key} {...rest}>
                  <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                    <Avatar
                      src={uploadUrl(option.avatar_url)}
                      alt=""
                      sx={{ width: 24, height: 24, fontSize: 12 }}
                    >
                      {nameInitial(option.name)}
                    </Avatar>
                    {option.name}
                  </Box>
                </li>
              )
            }}
            renderInput={(params) => <TextField {...params} label="选择神人" />}
          />
        ) : (
          <Stack spacing={2}>
            <TextField label="神人名称" value={name} onChange={(e) => setName(e.target.value)} fullWidth />
            <Button variant="outlined" component="label">
              {avatar ? avatar.name : '上传头像（可选）'}
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
        )}
      </DialogContent>
      <DialogActions>
        <Button onClick={onClose}>取消</Button>
        <Button variant="contained" disabled={busy} onClick={() => void submit()}>
          确认通过
        </Button>
      </DialogActions>
    </Dialog>
  )
}
