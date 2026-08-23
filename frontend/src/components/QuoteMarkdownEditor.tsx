import { useEffect, useRef, useState } from 'react'
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
  FormHelperText,
  FormLabel,
  IconButton,
  Radio,
  RadioGroup,
  FormControlLabel,
  Stack,
  TextField,
  Tooltip,
  Typography,
} from '@mui/material'
import FormatBoldIcon from '@mui/icons-material/FormatBold'
import FormatItalicIcon from '@mui/icons-material/FormatItalic'
import FormatQuoteIcon from '@mui/icons-material/FormatQuote'
import CodeIcon from '@mui/icons-material/Code'
import LinkIcon from '@mui/icons-material/Link'
import VisibilityIcon from '@mui/icons-material/Visibility'
import VisibilityOffIcon from '@mui/icons-material/VisibilityOff'
import { normalizePersons, publicApi } from '../api'
import type { Person, Quote } from '../api/types'
import { nameInitial, uploadUrl } from '../api/client'
import QuoteMarkdown from './QuoteMarkdown'

const EMPTY_PERSONS: Person[] = []

function toQuotedMarkdown(personName: string, quotedText: string): string {
  const lines = quotedText.replace(/\r\n/g, '\n').replace(/\n+$/g, '').split('\n')
  if (lines.length === 0 || lines.every((l) => !l.trim())) return ''
  return lines
    .map((line, i) => {
      if (i === 0) return `> **${personName}**：${line}`
      return line.length === 0 ? '>' : `> ${line}`
    })
    .join('\n')
}

export default function QuoteMarkdownEditor({
  value,
  onChange,
  persons = EMPTY_PERSONS,
  label = '语录内容',
  helperText,
  required = false,
  disabled = false,
  minRows = 5,
}: {
  value: string
  onChange: (value: string) => void
  persons?: Person[]
  label?: string
  helperText?: string
  required?: boolean
  disabled?: boolean
  minRows?: number
}) {
  const inputRef = useRef<HTMLTextAreaElement>(null)
  const [preview, setPreview] = useState(true)
  const [insertOpen, setInsertOpen] = useState(false)

  const applyAtSelection = (snippet: string, selectInside?: { start: number; end: number }) => {
    const el = inputRef.current
    const start = el?.selectionStart ?? value.length
    const end = el?.selectionEnd ?? value.length
    const next = value.slice(0, start) + snippet + value.slice(end)
    onChange(next)
    requestAnimationFrame(() => {
      el?.focus()
      const pos = start + (selectInside ? selectInside.start : snippet.length)
      const posEnd = start + (selectInside ? selectInside.end : snippet.length)
      el?.setSelectionRange(pos, posEnd)
    })
  }

  const wrapSelection = (before: string, after: string, placeholder: string) => {
    const el = inputRef.current
    const start = el?.selectionStart ?? value.length
    const end = el?.selectionEnd ?? value.length
    const selected = value.slice(start, end)
    const inner = selected || placeholder
    const snippet = before + inner + after
    onChange(value.slice(0, start) + snippet + value.slice(end))
    requestAnimationFrame(() => {
      el?.focus()
      const innerStart = start + before.length
      el?.setSelectionRange(innerStart, innerStart + inner.length)
    })
  }

  const prefixSelectedLines = (prefix: string) => {
    const el = inputRef.current
    const start = el?.selectionStart ?? 0
    const end = el?.selectionEnd ?? 0
    const lineStart = value.lastIndexOf('\n', Math.max(0, start - 1)) + 1
    const lineEnd = end === start ? value.length : end
    const block = value.slice(lineStart, lineEnd)
    const replaced = (block.length ? block : '引用')
      .split('\n')
      .map((l) => (l.startsWith(prefix) ? l : prefix + l))
      .join('\n')
    onChange(value.slice(0, lineStart) + replaced + value.slice(lineEnd))
    requestAnimationFrame(() => {
      el?.focus()
      el?.setSelectionRange(lineStart, lineStart + replaced.length)
    })
  }

  const insertQuoteBlock = (markdown: string) => {
    const el = inputRef.current
    const start = el?.selectionStart ?? value.length
    const end = el?.selectionEnd ?? value.length
    const before = value.slice(0, start)
    const after = value.slice(end)
    const lead = before.length > 0 && !before.endsWith('\n') ? '\n' : ''
    const trail = after.startsWith('\n') || after.length === 0 ? '\n' : '\n\n'
    const snippet = lead + markdown + trail
    onChange(before + snippet + after)
    requestAnimationFrame(() => {
      el?.focus()
      const pos = (before + snippet).length
      el?.setSelectionRange(pos, pos)
    })
  }

  return (
    <FormControl fullWidth required={required} disabled={disabled}>
      <FormLabel sx={{ mb: 0.75 }}>{label}</FormLabel>
      <Stack
        direction="row"
        spacing={0.25}
        sx={{ mb: 0.75, flexWrap: 'wrap', alignItems: 'center', gap: 0.25 }}
      >
        <Tooltip title="粗体">
          <IconButton
            type="button"
            size="small"
            disabled={disabled}
            onClick={() => wrapSelection('**', '**', '粗体')}
          >
            <FormatBoldIcon fontSize="small" />
          </IconButton>
        </Tooltip>
        <Tooltip title="斜体">
          <IconButton
            type="button"
            size="small"
            disabled={disabled}
            onClick={() => wrapSelection('*', '*', '斜体')}
          >
            <FormatItalicIcon fontSize="small" />
          </IconButton>
        </Tooltip>
        <Tooltip title="引用块">
          <IconButton
            type="button"
            size="small"
            disabled={disabled}
            onClick={() => prefixSelectedLines('> ')}
          >
            <FormatQuoteIcon fontSize="small" />
          </IconButton>
        </Tooltip>
        <Tooltip title="行内代码">
          <IconButton
            type="button"
            size="small"
            disabled={disabled}
            onClick={() => wrapSelection('`', '`', 'code')}
          >
            <CodeIcon fontSize="small" />
          </IconButton>
        </Tooltip>
        <Tooltip title="链接">
          <IconButton
            type="button"
            size="small"
            disabled={disabled}
            onClick={() => applyAtSelection('[文本](https://)', { start: 1, end: 3 })}
          >
            <LinkIcon fontSize="small" />
          </IconButton>
        </Tooltip>
        <Button
          type="button"
          size="small"
          variant="outlined"
          disabled={disabled}
          onClick={() => setInsertOpen(true)}
          sx={{ ml: 0.5 }}
        >
          插入引用
        </Button>
        <Box sx={{ flex: 1 }} />
        <Tooltip title={preview ? '隐藏预览' : '显示预览'}>
          <IconButton type="button" size="small" onClick={() => setPreview((v) => !v)}>
            {preview ? <VisibilityOffIcon fontSize="small" /> : <VisibilityIcon fontSize="small" />}
          </IconButton>
        </Tooltip>
      </Stack>
      <TextField
        value={value}
        onChange={(e) => onChange(e.target.value)}
        fullWidth
        required={required}
        disabled={disabled}
        multiline
        minRows={minRows}
        inputRef={inputRef}
        placeholder={'支持 Markdown。引用其他神人可用「插入引用」，例如：\n> **浪疯Koru**：不\n> 不是\n\n真的吗'}
        sx={{
          '& textarea': {
            fontFamily:
              'ui-monospace, SFMono-Regular, Menlo, Consolas, "Noto Sans SC", monospace',
            fontSize: 14,
            lineHeight: 1.55,
          },
        }}
      />
      {helperText ? <FormHelperText>{helperText}</FormHelperText> : null}
      {preview ? (
        <Box
          sx={{
            mt: 1.25,
            bgcolor: (theme) => theme.palette.mode === 'dark' ? '#2c2c2c' : '#e9eef5',
            color: 'text.primary',
            borderRadius: 1,
            px: 1.5,
            py: 1.25,
            minHeight: 48,
          }}
        >
          <Typography variant="caption" sx={{ color: 'text.secondary', display: 'block', mb: 0.75 }}>
            预览
          </Typography>
          {value.trim() ? (
            <QuoteMarkdown content={value} />
          ) : (
            <Typography variant="body2" sx={{ color: 'text.disabled' }}>
              输入内容后在此预览
            </Typography>
          )}
        </Box>
      ) : null}
      <InsertQuoteDialog
        open={insertOpen}
        persons={persons}
        onClose={() => setInsertOpen(false)}
        onInsert={(md) => {
          insertQuoteBlock(md)
          setInsertOpen(false)
        }}
      />
    </FormControl>
  )
}

function InsertQuoteDialog({
  open,
  persons: personsProp,
  onClose,
  onInsert,
}: {
  open: boolean
  persons: Person[]
  onClose: () => void
  onInsert: (markdown: string) => void
}) {
  const [persons, setPersons] = useState<Person[]>(personsProp)
  const [person, setPerson] = useState<Person | null>(null)
  const [mode, setMode] = useState<'existing' | 'custom'>('custom')
  const [quotes, setQuotes] = useState<Quote[]>([])
  const [selectedQuote, setSelectedQuote] = useState<Quote | null>(null)
  const [customText, setCustomText] = useState('')
  const [loadingQuotes, setLoadingQuotes] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const personsRef = useRef(personsProp)
  personsRef.current = personsProp

  useEffect(() => {
    if (!open) return
    setPerson(null)
    setMode('custom')
    setQuotes([])
    setSelectedQuote(null)
    setCustomText('')
    setError(null)
    const list = personsRef.current
    if (list.length > 0) {
      setPersons(list)
      return
    }
    publicApi
      .getPersons()
      .then((d) => setPersons(normalizePersons(d)))
      .catch(() => setPersons([]))
  }, [open])

  useEffect(() => {
    if (!open || !person) {
      setQuotes([])
      setSelectedQuote(null)
      return
    }
    let cancelled = false
    setLoadingQuotes(true)
    publicApi
      .getQuotes(1, 50, person.id)
      .then((data) => {
        if (cancelled) return
        setQuotes(data.items)
        setSelectedQuote(null)
        setMode(data.items.length > 0 ? 'existing' : 'custom')
      })
      .catch(() => {
        if (!cancelled) {
          setQuotes([])
          setMode('custom')
        }
      })
      .finally(() => {
        if (!cancelled) setLoadingQuotes(false)
      })
    return () => {
      cancelled = true
    }
  }, [open, person])

  const quotedText =
    mode === 'existing' ? (selectedQuote?.content ?? '') : customText

  const canInsert = person != null && quotedText.trim().length > 0

  const confirm = () => {
    if (!person) {
      setError('请选择要引用的神人')
      return
    }
    const md = toQuotedMarkdown(person.name, quotedText)
    if (!md) {
      setError('请填写或选择被引用的内容')
      return
    }
    onInsert(md)
  }

  return (
    <Dialog open={open} onClose={onClose} fullWidth maxWidth="sm">
      <DialogTitle>插入引用</DialogTitle>
      <DialogContent>
        <Stack spacing={2} sx={{ mt: 1 }}>
          {error ? (
            <Alert severity="error" onClose={() => setError(null)}>
              {error}
            </Alert>
          ) : null}
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
            renderInput={(params) => (
              <TextField {...params} label="引用哪位神人" required placeholder="搜索或选择" />
            )}
          />
          <RadioGroup
            row
            value={mode}
            onChange={(_, v) => setMode(v as 'existing' | 'custom')}
          >
            <FormControlLabel
              value="existing"
              control={<Radio />}
              label="选择已有语录"
              disabled={!person || quotes.length === 0}
            />
            <FormControlLabel value="custom" control={<Radio />} label="自行填写" />
          </RadioGroup>
          {mode === 'existing' ? (
            loadingQuotes ? (
              <Box sx={{ display: 'flex', justifyContent: 'center', py: 2 }}>
                <CircularProgress size={22} />
              </Box>
            ) : quotes.length === 0 ? (
              <Typography variant="body2" color="text.secondary">
                该神人暂无已通过语录，请自行填写引用内容。
              </Typography>
            ) : (
              <Stack
                sx={{
                  maxHeight: 220,
                  overflow: 'auto',
                  border: '1px solid',
                  borderColor: 'divider',
                  borderRadius: 1,
                }}
              >
                {quotes.map((q) => (
                  <Box
                    key={q.id}
                    role="button"
                    tabIndex={0}
                    onClick={() => setSelectedQuote(q)}
                    onKeyDown={(e) => {
                      if (e.key === 'Enter' || e.key === ' ') {
                        e.preventDefault()
                        setSelectedQuote(q)
                      }
                    }}
                    sx={{
                      px: 1.25,
                      py: 1,
                      cursor: 'pointer',
                      borderBottom: '1px solid',
                      borderBottomColor: 'divider',
                      bgcolor: selectedQuote?.id === q.id ? 'action.selected' : 'transparent',
                      '&:hover': { bgcolor: 'action.hover' },
                      '&:last-child': { borderBottom: 0 },
                    }}
                  >
                    <Typography
                      variant="body2"
                      sx={{
                        whiteSpace: 'pre-wrap',
                        display: '-webkit-box',
                        WebkitLineClamp: 3,
                        WebkitBoxOrient: 'vertical',
                        overflow: 'hidden',
                      }}
                    >
                      {q.content}
                    </Typography>
                  </Box>
                ))}
              </Stack>
            )
          ) : (
            <TextField
              label="被引用的原文"
              value={customText}
              onChange={(e) => setCustomText(e.target.value)}
              fullWidth
              multiline
              minRows={3}
              placeholder="每行一句，将插入为 Markdown 引用块"
            />
          )}
        </Stack>
      </DialogContent>
      <DialogActions>
        <Button type="button" onClick={onClose}>
          取消
        </Button>
        <Button type="button" variant="contained" disabled={!canInsert} onClick={confirm}>
          插入
        </Button>
      </DialogActions>
    </Dialog>
  )
}
