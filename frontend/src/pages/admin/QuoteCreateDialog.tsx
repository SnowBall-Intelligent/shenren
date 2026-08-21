import { useEffect, useState } from 'react'
import {
  Alert,
  Autocomplete,
  Avatar,
  Box,
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  TextField,
} from '@mui/material'
import { adminApi } from '../../api'
import type { Person, Quote } from '../../api/types'
import { nameInitial, uploadUrl } from '../../api/client'
import QuoteMarkdownEditor from '../../components/QuoteMarkdownEditor'
import { useToast } from '../../components/AppToast'

export default function QuoteCreateDialog({
  open,
  person = null,
  quote = null,
  persons = [],
  onClose,
  onCreated,
}: {
  open: boolean
  person?: Person | null
  quote?: Quote | null
  persons?: Person[]
  onClose: () => void
  onCreated: () => void | Promise<void>
}) {
  const toast = useToast()
  const editing = quote != null
  const locked = person != null && !editing
  const [selected, setSelected] = useState<Person | null>(null)
  const [content, setContent] = useState('')
  const [source, setSource] = useState('')
  const [busy, setBusy] = useState(false)
  const [formError, setFormError] = useState<string | null>(null)

  useEffect(() => {
    if (open) {
      const fromQuote =
        quote?.person_id != null
          ? (persons.find((p) => p.id === quote.person_id) ??
            (quote.person
              ? {
                  id: quote.person.id,
                  name: quote.person.name,
                  avatar_url: quote.person.avatar_url,
                }
              : null))
          : null
      setSelected(person ?? fromQuote)
      setContent(quote?.content ?? '')
      setSource(quote?.source ?? '')
      setFormError(null)
    }
  }, [open, person, quote, persons])

  const submit = async () => {
    if (!selected) {
      setFormError('请选择神人')
      return
    }
    const trimmed = content.trim()
    if (!trimmed) {
      setFormError('请填写语录内容')
      return
    }
    setBusy(true)
    setFormError(null)
    try {
      const payload = {
        person_id: selected.id,
        content: trimmed,
        source: source.trim() || null,
      }
      const data = editing
        ? await adminApi.updateQuote(quote.id, payload)
        : await adminApi.createQuote(payload)
      toast.fromSuccess(data)
      await onCreated()
    } catch (e) {
      toast.fromError(e)
    } finally {
      setBusy(false)
    }
  }

  return (
    <Dialog open={open} onClose={onClose} fullWidth maxWidth="md">
      <DialogTitle>{editing ? '修改语录' : '添加语录'}</DialogTitle>
      <DialogContent>
        <Box sx={{ display: 'flex', flexDirection: 'column', gap: 2, mt: 1 }}>
          {formError ? (
            <Alert severity="error" onClose={() => setFormError(null)}>
              {formError}
            </Alert>
          ) : null}
          {locked ? (
            <TextField label="神人" value={person?.name ?? ''} fullWidth disabled />
          ) : (
            <Autocomplete
              options={persons}
              value={selected}
              onChange={(_, v) => setSelected(v)}
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
              renderInput={(params) => (
                <TextField
                  {...params}
                  label="神人"
                  required
                  placeholder="搜索或选择"
                  helperText={persons.length === 0 ? '暂无神人，请先到神人页新增' : undefined}
                />
              )}
            />
          )}
          <QuoteMarkdownEditor
            label="语录内容"
            value={content}
            onChange={setContent}
            persons={persons}
            required
            helperText={
              editing ? '内容以 Markdown 保存。' : '添加后立即通过，无需审核。内容以 Markdown 保存。'
            }
          />
          <TextField
            label="来源 / 备注（可选）"
            value={source}
            onChange={(e) => setSource(e.target.value)}
            fullWidth
            slotProps={{ htmlInput: { maxLength: 500 } }}
          />
        </Box>
      </DialogContent>
      <DialogActions>
        <Button onClick={onClose}>取消</Button>
        <Button variant="contained" disabled={busy} onClick={() => void submit()}>
          {editing ? '保存' : '添加'}
        </Button>
      </DialogActions>
    </Dialog>
  )
}
