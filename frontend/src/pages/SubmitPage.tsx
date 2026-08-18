import { useEffect, useMemo, useState } from 'react'
import { useOutletContext } from 'react-router-dom'
import {
  Alert,
  Autocomplete,
  Box,
  Button,
  FormControlLabel,
  Radio,
  RadioGroup,
  Stack,
  TextField,
  Typography,
} from '@mui/material'
import { normalizePersons, publicApi } from '../api'
import type { Person, SiteInfo } from '../api/types'
import { ApiError } from '../api/client'

type Mode = 'existing' | 'propose'

export default function SubmitPage() {
  const { site } = useOutletContext<{ site: SiteInfo | null }>()
  const allowPropose = site?.allow_propose_person ?? false

  const [persons, setPersons] = useState<Person[]>([])
  const [mode, setMode] = useState<Mode>('existing')
  const [person, setPerson] = useState<Person | null>(null)
  const [proposedName, setProposedName] = useState('')
  const [content, setContent] = useState('')
  const [source, setSource] = useState('')
  const [submitting, setSubmitting] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [success, setSuccess] = useState(false)

  useEffect(() => {
    publicApi
      .getPersons()
      .then((data) => setPersons(normalizePersons(data)))
      .catch(() => setPersons([]))
  }, [])

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
    setError(null)
    setSuccess(false)
    try {
      await publicApi.submit({
        person_id: mode === 'existing' ? person!.id : null,
        proposed_person_name: mode === 'propose' ? proposedName.trim() : null,
        content: content.trim(),
        source: source.trim() || null,
      })
      setSuccess(true)
      setContent('')
      setSource('')
      setProposedName('')
      setPerson(null)
    } catch (err) {
      setError(err instanceof ApiError ? err.message : '投稿失败')
    } finally {
      setSubmitting(false)
    }
  }

  return (
    <Box component="form" onSubmit={(e) => void handleSubmit(e)} sx={{ maxWidth: 560, mx: 'auto' }}>
      <Typography variant="h5" sx={{ fontWeight: 700, mb: 0.5 }}>
        投稿
      </Typography>
      <Typography variant="body2" color="text.secondary" sx={{ mb: 3 }}>
        提交后进入待审核，通过后才会显示在首页。
      </Typography>

      {success ? (
        <Alert severity="success" sx={{ mb: 2 }} onClose={() => setSuccess(false)}>
          投稿成功，请等待审核。
        </Alert>
      ) : null}
      {error ? (
        <Alert severity="error" sx={{ mb: 2 }} onClose={() => setError(null)}>
          {error}
        </Alert>
      ) : null}

      <Stack spacing={2.5}>
        {allowPropose ? (
          <RadioGroup
            row
            value={mode}
            onChange={(_, v) => setMode(v as Mode)}
          >
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

        <TextField
          label="言论内容"
          required
          multiline
          minRows={4}
          value={content}
          onChange={(e) => setContent(e.target.value)}
        />

        <TextField
          label="来源 / 备注（可选）"
          value={source}
          onChange={(e) => setSource(e.target.value)}
        />

        <Button type="submit" variant="contained" size="large" disabled={!canSubmit || submitting}>
          {submitting ? '提交中…' : '提交'}
        </Button>
      </Stack>
    </Box>
  )
}
