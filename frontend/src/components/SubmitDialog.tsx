import { useEffect, useMemo, useState } from 'react'
import {
  Autocomplete,
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  FormControlLabel,
  Radio,
  RadioGroup,
  Stack,
  TextField,
  Typography,
} from '@mui/material'
import { normalizePersons, publicApi } from '../api'
import type { Person, SiteInfo } from '../api/types'
import QuoteMarkdownEditor from './QuoteMarkdownEditor'
import { useToast } from './AppToast'

type Mode = 'existing' | 'propose'

export default function SubmitDialog({
  open,
  onClose,
  site,
}: {
  open: boolean
  onClose: () => void
  site: SiteInfo | null
}) {
  const allowPropose = site?.allow_propose_person ?? false
  const toast = useToast()

  const [persons, setPersons] = useState<Person[]>([])
  const [mode, setMode] = useState<Mode>('existing')
  const [person, setPerson] = useState<Person | null>(null)
  const [proposedName, setProposedName] = useState('')
  const [content, setContent] = useState('')
  const [source, setSource] = useState('')
  const [submitting, setSubmitting] = useState(false)

  useEffect(() => {
    if (!open) return
    publicApi
      .getPersons()
      .then((data) => setPersons(normalizePersons(data)))
      .catch(() => setPersons([]))
  }, [open])

  useEffect(() => {
    if (!allowPropose && mode === 'propose') {
      setMode('existing')
    }
  }, [allowPropose, mode])

  const canSubmit = useMemo(() => {
    if (!content.trim()) return false
    if (mode === 'existing') return person != null
    return proposedName.trim().length > 0
  }, [content, mode, person, proposedName])

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!canSubmit) return
    setSubmitting(true)
    try {
      const data = await publicApi.submit({
        person_id: mode === 'existing' ? person!.id : null,
        proposed_person_name: mode === 'propose' ? proposedName.trim() : null,
        content: content.trim(),
        source: source.trim() || null,
      })
      toast.fromSuccess(data)
      setContent('')
      setSource('')
      setProposedName('')
      setPerson(null)
      onClose()
    } catch (err) {
      toast.fromError(err)
    } finally {
      setSubmitting(false)
    }
  }

  return (
    <Dialog open={open} onClose={onClose} fullWidth maxWidth="sm" scroll="paper">
      <form onSubmit={(e) => void handleSubmit(e)}>
        <DialogTitle>投稿</DialogTitle>
        <DialogContent dividers>
          <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
            提交后进入待审核，通过后才会显示在首页。
          </Typography>

          <Stack spacing={2.5}>
            {allowPropose ? (
              <RadioGroup row value={mode} onChange={(_, v) => setMode(v as Mode)}>
                <FormControlLabel value="existing" control={<Radio />} label="选择已有神人" />
                <FormControlLabel value="propose" control={<Radio />} label="提出新神人" />
              </RadioGroup>
            ) : null}

            {mode === 'existing' ? (
              <Autocomplete
                options={persons}
                value={person}
                onChange={(_, v) => setPerson(v)}
                getOptionLabel={(o) => o.name}
                isOptionEqualToValue={(a, b) => a.id === b.id}
                renderInput={(params) => (
                  <TextField {...params} label="神人" required placeholder="搜索或选择" />
                )}
              />
            ) : (
              <TextField
                label="新神人名称"
                required
                value={proposedName}
                onChange={(e) => setProposedName(e.target.value)}
                helperText="审核通过后会建立神人档案；未提供头像时使用名称首字生成"
              />
            )}

            <QuoteMarkdownEditor
              label="言论内容"
              required
              value={content}
              onChange={setContent}
              persons={persons}
              helperText="支持 Markdown；可用「插入引用」引用其他神人的话。"
            />

            <TextField
              label="来源 / 备注（可选）"
              value={source}
              onChange={(e) => setSource(e.target.value)}
            />
          </Stack>
        </DialogContent>
        <DialogActions>
          <Button onClick={onClose}>关闭</Button>
          <Button type="submit" variant="contained" disabled={!canSubmit || submitting}>
            {submitting ? '提交中…' : '提交'}
          </Button>
        </DialogActions>
      </form>
    </Dialog>
  )
}
