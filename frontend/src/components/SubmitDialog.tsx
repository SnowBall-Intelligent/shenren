import { useCallback, useEffect, useMemo, useState } from 'react'
import {
  Autocomplete,
  Avatar,
  Box,
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
import type { CaptchaPayload, Person, SiteInfo } from '../api/types'
import QuoteMarkdownEditor from './QuoteMarkdownEditor'
import CaptchaWidget, { publicCaptchaList } from './CaptchaWidget'
import { useToast } from './AppToast'
import { ApiError, nameInitial, uploadUrl } from '../api/client'

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
  const [captcha, setCaptcha] = useState<CaptchaPayload | null>(null)
  const [captchaKey, setCaptchaKey] = useState(0)
  const [skipSignal, setSkipSignal] = useState(0)
  const captchaProviders = useMemo(() => publicCaptchaList(site?.captcha), [site?.captcha])
  const captchaRequired = captchaProviders.length > 0

  const handleCaptchaChange = useCallback((payload: CaptchaPayload | null) => {
    setCaptcha(payload)
  }, [])

  const handleCaptchaExhausted = useCallback((message: string) => {
    toast.error(message)
  }, [toast])

  const resetCaptcha = () => {
    setCaptcha(null)
    setCaptchaKey((k) => k + 1)
  }

  useEffect(() => {
    if (!open) {
      setCaptcha(null)
      setSkipSignal(0)
    }
  }, [open])

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
    if (captchaRequired && !captcha) return false
    if (mode === 'existing') return person != null
    return proposedName.trim().length > 0
  }, [captcha, captchaRequired, content, mode, person, proposedName])

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
        captcha: captcha ?? undefined,
      })
      toast.fromSuccess(data)
      setContent('')
      setSource('')
      setProposedName('')
      setPerson(null)
      setCaptcha(null)
      onClose()
    } catch (err) {
      const fallback = err instanceof ApiError && err.body?.captcha_fallback
      if (fallback) {
        setCaptcha(null)
        setSkipSignal((n) => n + 1)
      } else {
        toast.fromError(err)
        resetCaptcha()
      }
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
              <Box sx={{ display: 'flex', alignItems: 'center', gap: 1.25 }}>
                <Avatar
                  src={person ? uploadUrl(person.avatar_url) : undefined}
                  alt=""
                  sx={{ width: 40, height: 40, fontSize: 16, flexShrink: 0 }}
                >
                  {person ? nameInitial(person.name) : '?'}
                </Avatar>
                <Autocomplete
                  sx={{ flex: 1 }}
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
                  renderInput={(params) => (
                    <TextField {...params} label="神人" required placeholder="搜索或选择" />
                  )}
                />
              </Box>
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

            {open && captchaRequired ? (
              <CaptchaWidget
                key={captchaKey}
                providers={captchaProviders}
                skipSignal={skipSignal}
                onChange={handleCaptchaChange}
                onExhausted={handleCaptchaExhausted}
              />
            ) : null}
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
