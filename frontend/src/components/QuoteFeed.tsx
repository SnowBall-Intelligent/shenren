import { useCallback, useEffect, useRef, useState } from 'react'
import { Alert, Box, Button, CircularProgress, Stack, Typography } from '@mui/material'
import { publicApi } from '../api'
import type { Quote } from '../api/types'
import QuoteBubble from './QuoteBubble'
import QuoteTimeline from './QuoteTimeline'
import { ApiError } from '../api/client'

const PAGE_SIZE = 20

export default function QuoteFeed() {
  const [quotes, setQuotes] = useState<Quote[]>([])
  const [page, setPage] = useState(1)
  const [total, setTotal] = useState(0)
  const [loading, setLoading] = useState(true)
  const [loadingMore, setLoadingMore] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const sentinelRef = useRef<HTMLDivElement | null>(null)
  const quoteRefs = useRef<Array<HTMLDivElement | null>>([])
  const inFlight = useRef(false)
  const [activeIndex, setActiveIndex] = useState(0)

  const loadPage = useCallback(async (nextPage: number, append: boolean) => {
    if (inFlight.current) return
    inFlight.current = true
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
      inFlight.current = false
    }
  }, [])

  useEffect(() => {
    void loadPage(1, false)
  }, [loadPage])

  const hasMore = quotes.length < total

  useEffect(() => {
    const el = sentinelRef.current
    if (!el || !hasMore || loading) return
    const io = new IntersectionObserver(
      (entries) => {
        if (entries[0]?.isIntersecting && hasMore && !inFlight.current) {
          void loadPage(page + 1, true)
        }
      },
      { root: null, rootMargin: '240px', threshold: 0 },
    )
    io.observe(el)
    return () => io.disconnect()
  }, [hasMore, loading, loadPage, page])

  useEffect(() => {
    let frame = 0
    const updateActive = () => {
      frame = 0
      const headerOffset = window.innerWidth < 600 ? 72 : 80
      let bestIndex = 0
      let bestDistance = Number.POSITIVE_INFINITY
      quoteRefs.current.slice(0, quotes.length).forEach((node, index) => {
        if (!node) return
        const rect = node.getBoundingClientRect()
        if (rect.bottom < headerOffset || rect.top > window.innerHeight) return
        const distance = Math.abs(rect.top - headerOffset)
        if (distance < bestDistance) {
          bestDistance = distance
          bestIndex = index
        }
      })
      if (bestDistance !== Number.POSITIVE_INFINITY) setActiveIndex(bestIndex)
    }
    const schedule = () => {
      if (!frame) frame = window.requestAnimationFrame(updateActive)
    }
    updateActive()
    window.addEventListener('scroll', schedule, { passive: true })
    window.addEventListener('resize', schedule)
    return () => {
      window.removeEventListener('scroll', schedule)
      window.removeEventListener('resize', schedule)
      if (frame) window.cancelAnimationFrame(frame)
    }
  }, [quotes.length])

  const locateQuote = useCallback((index: number) => {
    const node = quoteRefs.current[index]
    if (!node) return
    const reduceMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches
    const rect = node.getBoundingClientRect()
    const targetTop = window.scrollY + rect.top - (window.innerHeight - rect.height) / 2
    window.scrollTo({
      top: Math.max(0, targetTop),
      behavior: reduceMotion ? 'auto' : 'smooth',
    })
    setActiveIndex(index)
  }, [])

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
    <Box
      sx={{
        display: 'grid',
        gridTemplateColumns: { xs: '24px minmax(0, 1fr)', sm: '32px minmax(0, 1fr)' },
        columnGap: { xs: 0.5, sm: 1 },
        alignItems: 'start',
      }}
    >
      <QuoteTimeline quotes={quotes} activeIndex={activeIndex} onLocate={locateQuote} />
      <Stack spacing={0} sx={{ minWidth: 0 }}>
        {quotes.map((quote, index) => {
          const prev = index > 0 ? quotes[index - 1] : null
          const quotePersonId = quote.person?.id ?? quote.person_id ?? null
          const prevPersonId = prev ? (prev.person?.id ?? prev.person_id ?? null) : null
          const samePerson = prev != null && quotePersonId != null && quotePersonId === prevPersonId
          return (
            <Box
              key={quote.id}
              ref={(node: HTMLDivElement | null) => {
                quoteRefs.current[index] = node
              }}
              data-testid={`quote-item-${index + 1}`}
            >
              <QuoteBubble
                quote={quote}
                sequence={index + 1}
                showIdentity={!samePerson}
                tightGap={samePerson}
              />
            </Box>
          )
        })}

        <Box ref={sentinelRef} sx={{ display: 'flex', justifyContent: 'center', py: 3 }}>
          {loadingMore ? <CircularProgress size={22} /> : null}
          {!hasMore && !loadingMore ? (
            <Typography variant="caption" color="text.secondary">
              没有更多了
            </Typography>
          ) : null}
        </Box>
      </Stack>
    </Box>
  )
}
