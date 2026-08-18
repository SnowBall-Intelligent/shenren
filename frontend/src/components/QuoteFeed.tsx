import { useCallback, useEffect, useState } from 'react'
import { Alert, Box, Button, CircularProgress, Stack, Typography } from '@mui/material'
import { publicApi } from '../api'
import type { Quote } from '../api/types'
import QuoteBubble from './QuoteBubble'
import { ApiError } from '../api/client'

const PAGE_SIZE = 20

export default function QuoteFeed() {
  const [quotes, setQuotes] = useState<Quote[]>([])
  const [page, setPage] = useState(1)
  const [total, setTotal] = useState(0)
  const [loading, setLoading] = useState(true)
  const [loadingMore, setLoadingMore] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const loadPage = useCallback(async (nextPage: number, append: boolean) => {
    if (append) setLoadingMore(true)
    else setLoading(true)
    setError(null)
    try {
      const data = await publicApi.getQuotes(nextPage, PAGE_SIZE)
      setTotal(data.total)
      setPage(data.page)
      setQuotes((prev) => (append ? [...prev, ...data.items] : data.items))
    } catch (e) {
      const msg = e instanceof ApiError ? e.message : '加载失败'
      setError(msg)
    } finally {
      setLoading(false)
      setLoadingMore(false)
    }
  }, [])

  useEffect(() => {
    void loadPage(1, false)
  }, [loadPage])

  const hasMore = quotes.length < total

  if (loading) {
    return (
      <Box sx={{ display: 'flex', justifyContent: 'center', py: 6 }}>
        <CircularProgress size={28} />
      </Box>
    )
  }

  if (error && quotes.length === 0) {
    return (
      <Alert
        severity="error"
        action={
          <Button color="inherit" size="small" onClick={() => void loadPage(1, false)}>
            重试
          </Button>
        }
      >
        {error}
      </Alert>
    )
  }

  if (quotes.length === 0) {
    return (
      <Typography color="text.secondary" sx={{ textAlign: 'center', py: 6 }}>
        暂无已收录言论
      </Typography>
    )
  }

  return (
    <Stack spacing={0}>
      {quotes.map((quote, index) => {
        const prev = index > 0 ? quotes[index - 1] : null
        const quotePersonId = quote.person?.id ?? quote.person_id ?? null
        const prevPersonId = prev ? (prev.person?.id ?? prev.person_id ?? null) : null
        const samePerson =
          prev != null && quotePersonId != null && quotePersonId === prevPersonId
        return (
          <QuoteBubble
            key={quote.id}
            quote={quote}
            showIdentity={!samePerson}
            tightGap={samePerson}
          />
        )
      })}

      {hasMore ? (
        <Box sx={{ display: 'flex', justifyContent: 'center', pt: 3, pb: 1 }}>
          <Button
            variant="outlined"
            disabled={loadingMore}
            onClick={() => void loadPage(page + 1, true)}
          >
            {loadingMore ? <CircularProgress size={20} /> : '加载更多'}
          </Button>
        </Box>
      ) : (
        <Typography variant="caption" color="text.secondary" sx={{ textAlign: 'center', pt: 3 }}>
          没有更多了
        </Typography>
      )}
    </Stack>
  )
}
