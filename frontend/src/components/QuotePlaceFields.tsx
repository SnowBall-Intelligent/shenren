import { useEffect, useMemo, useState } from 'react'
import {
  Autocomplete,
  Avatar,
  Box,
  FormControl,
  FormControlLabel,
  FormLabel,
  Radio,
  RadioGroup,
  TextField,
  Typography,
} from '@mui/material'
import { adminApi, publicApi } from '../api'
import { nameInitial, uploadUrl } from '../api/client'
import type { Quote } from '../api/types'

export function quoteLabel(q: Quote): string {
  const name = q.person?.name ?? '未知神人'
  const text = q.content.replace(/[#>*_`[\]]/g, '').replace(/\s+/g, ' ').trim()
  const snippet = text.length > 28 ? `${text.slice(0, 28)}…` : text
  return snippet ? `${name}：${snippet}` : name
}

function quoteSnippet(q: Quote): string {
  return q.content.replace(/[#>*_`[\]]/g, '').replace(/\s+/g, ' ').trim()
}

export function toDatetimeLocal(iso?: string): string {
  const d = iso ? new Date(iso) : new Date()
  if (Number.isNaN(d.getTime())) return ''
  const pad2 = (n: number) => String(n).padStart(2, '0')
  return `${d.getFullYear()}-${pad2(d.getMonth() + 1)}-${pad2(d.getDate())}T${pad2(d.getHours())}:${pad2(d.getMinutes())}`
}

export function fromDatetimeLocal(value: string): string | null {
  const trimmed = value.trim()
  if (!trimmed) return null
  const d = new Date(trimmed)
  if (Number.isNaN(d.getTime())) return null
  return d.toISOString()
}

export default function QuotePlaceFields({
  source,
  enabled = true,
  excludeId,
  pinnedOnly,
  anchor,
  onAnchorChange,
  place,
  onPlaceChange,
  publishedAt,
  onPublishedAtChange,
  keepOrderHint,
  publishedHint,
}: {
  source: 'public' | 'admin'
  enabled?: boolean
  excludeId?: string
  pinnedOnly?: boolean
  anchor: Quote | null
  onAnchorChange: (q: Quote | null) => void
  place: 'before' | 'after'
  onPlaceChange: (p: 'before' | 'after') => void
  publishedAt: string
  onPublishedAtChange: (v: string) => void
  keepOrderHint?: string
  publishedHint?: string
}) {
  const [quotes, setQuotes] = useState<Quote[]>([])
  const [loading, setLoading] = useState(false)
  const [search, setSearch] = useState('')
  const [inputValue, setInputValue] = useState('')

  useEffect(() => {
    if (!enabled) {
      setSearch('')
      setInputValue('')
      return
    }
    setSearch('')
    setInputValue('')
  }, [enabled])

  useEffect(() => {
    if (!enabled) return
    let cancelled = false
    setLoading(true)
    const timer = window.setTimeout(() => {
      const q = search.trim() || undefined
      const pageSize = q ? 20 : 10
      const req =
        source === 'admin'
          ? adminApi.listQuotes({
              status: 'approved',
              page: 1,
              page_size: pageSize,
              q,
              pinned: pinnedOnly,
            })
          : publicApi.getQuotes(1, pageSize, undefined, {
              q,
              pinned: pinnedOnly,
            })
      req
        .then((d) => {
          if (!cancelled) setQuotes(d.items)
        })
        .catch(() => {
          if (!cancelled) setQuotes([])
        })
        .finally(() => {
          if (!cancelled) setLoading(false)
        })
    }, 250)
    return () => {
      cancelled = true
      window.clearTimeout(timer)
    }
  }, [enabled, source, search, pinnedOnly])

  const options = useMemo(() => {
    const list = quotes.filter((q) => excludeId == null || q.id !== excludeId)
    if (anchor && (excludeId == null || anchor.id !== excludeId) && !list.some((q) => q.id === anchor.id)) {
      return [anchor, ...list]
    }
    return list
  }, [quotes, excludeId, anchor])

  return (
    <>
      <FormControl>
        <FormLabel>位置</FormLabel>
        <RadioGroup
          row
          value={place}
          onChange={(e) => onPlaceChange(e.target.value as 'before' | 'after')}
        >
          <FormControlLabel value="before" control={<Radio />} label="排在某条前面" />
          <FormControlLabel value="after" control={<Radio />} label="排在某条后面" />
        </RadioGroup>
      </FormControl>
      <Autocomplete
        options={options}
        value={anchor}
        inputValue={inputValue}
        onChange={(_, v) => onAnchorChange(v)}
        onInputChange={(_, v, reason) => {
          setInputValue(v)
          if (reason === 'input' || reason === 'clear') setSearch(v)
        }}
        filterOptions={(x) => x}
        loading={loading}
        getOptionLabel={(o) => quoteLabel(o)}
        isOptionEqualToValue={(a, b) => a.id === b.id}
        noOptionsText={search.trim() ? '没有匹配的语录' : '暂无语录'}
        renderOption={(props, option) => {
          const { key, ...rest } = props
          const name = option.person?.name ?? '未知神人'
          const snippet = quoteSnippet(option)
          return (
            <li key={key} {...rest}>
              <Box sx={{ display: 'flex', alignItems: 'center', gap: 1.25, minWidth: 0, py: 0.25 }}>
                <Avatar
                  src={option.person ? uploadUrl(option.person.avatar_url) : undefined}
                  alt=""
                  sx={{ width: 28, height: 28, fontSize: 13, flexShrink: 0 }}
                >
                  {nameInitial(name)}
                </Avatar>
                <Box sx={{ minWidth: 0 }}>
                  <Typography variant="body2" noWrap>
                    {name}
                  </Typography>
                  {snippet ? (
                    <Typography variant="caption" color="text.secondary" noWrap sx={{ display: 'block' }}>
                      {snippet}
                    </Typography>
                  ) : null}
                </Box>
              </Box>
            </li>
          )
        }}
        renderInput={(params) => (
          <TextField
            {...params}
            label={place === 'before' ? '插在哪条前面' : '插在哪条后面'}
            placeholder="搜索神人或内容"
            helperText={keepOrderHint ?? '默认展示前 10 条；不选则按发布时间插入现有顺序。'}
            slotProps={{
              ...params.slotProps,
              input: {
                ...params.slotProps.input,
                startAdornment: (
                  <>
                    {anchor?.person ? (
                      <Avatar
                        src={uploadUrl(anchor.person.avatar_url)}
                        alt=""
                        sx={{ width: 24, height: 24, ml: 0.5, mr: 0.5, fontSize: 12 }}
                      >
                        {nameInitial(anchor.person.name)}
                      </Avatar>
                    ) : null}
                    {params.slotProps.input.startAdornment}
                  </>
                ),
              },
            }}
          />
        )}
      />
      <TextField
        label="发布时间"
        type="datetime-local"
        value={publishedAt}
        onChange={(e) => onPublishedAtChange(e.target.value)}
        helperText={publishedHint ?? '可填过去或未来；不选手动插位时按此时间插入。'}
        slotProps={{ inputLabel: { shrink: true } }}
      />
    </>
  )
}
